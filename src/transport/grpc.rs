use std::time::Instant;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures_util::{stream, StreamExt};
use prost_reflect::prost::Message;
use prost_reflect::prost_types::{FileDescriptorProto, FileDescriptorSet, ServiceDescriptorProto};
use prost_reflect::{DescriptorPool, DynamicMessage, FieldDescriptor, Kind, MessageDescriptor, MethodDescriptor};
use serde_json::{Map, Value as JsonValue};
use serde::de::DeserializeSeed;
use serde::Serialize;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::metadata::{MetadataKey, MetadataMap, MetadataValue};
use tonic::transport::Channel;
use tonic::{Request, Status};
use tonic_reflection::pb::v1::server_reflection_client::ServerReflectionClient;
use tonic_reflection::pb::v1::server_reflection_request::MessageRequest;
use tonic_reflection::pb::v1::server_reflection_response::MessageResponse;
use tonic_reflection::pb::v1::ServerReflectionRequest;

use crate::domain::{GrpcMethodInfo, HttpTiming, KeyValueField, ResponseBody};
use crate::transport::http::{HttpRequestResult, HttpResponse};

#[derive(Debug, Clone, Default)]
struct BytesCodec;

impl Codec for BytesCodec {
    type Encode = Bytes;
    type Decode = Bytes;
    type Encoder = BytesEncoder;
    type Decoder = BytesDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        BytesEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        BytesDecoder
    }
}

#[derive(Debug, Clone, Default)]
struct BytesEncoder;

impl Encoder for BytesEncoder {
    type Item = Bytes;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        dst.put(item);
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct BytesDecoder;

impl Decoder for BytesDecoder {
    type Item = Bytes;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        if !src.has_remaining() {
            return Ok(None);
        }
        let mut buf = BytesMut::with_capacity(src.remaining());
        buf.put(src);
        Ok(Some(buf.freeze()))
    }
}

pub async fn discover_grpc_methods(endpoint: &str) -> Result<Vec<GrpcMethodInfo>, String> {
    let channel = connect(endpoint).await?;
    let mut client = ServerReflectionClient::new(channel);
    let services = list_services(&mut client).await?;

    let mut methods = Vec::new();
    for service in services {
        let Some(file) = fetch_file_for_symbol(&mut client, &service).await? else {
            continue;
        };
        methods.extend(methods_for_service(&file, &service));
    }

    methods.sort_by_key(|left| left.label());
    methods.dedup_by(|left, right| left.path() == right.path());
    Ok(methods)
}

pub async fn generate_grpc_request_template(
    endpoint: &str,
    service: &str,
    method: &str,
) -> Result<String, String> {
    let method_descriptor = resolve_grpc_method(endpoint, service, method).await?;
    message_descriptor_to_template_json(method_descriptor.input())
}

pub async fn send_grpc_request(
    endpoint: &str,
    service: &str,
    method: &str,
    metadata: Vec<KeyValueField>,
    json_body: &str,
) -> HttpRequestResult {
    let mut timing = HttpTiming::default();
    let prepare_started = Instant::now();

    if service.trim().is_empty() || method.trim().is_empty() {
        timing.prepare_request_ms = prepare_started.elapsed().as_millis();
        return HttpRequestResult {
            timing,
            response: Err("gRPC service and method are required".into()),
        };
    }

    let result = async {
        let method_descriptor = resolve_grpc_method(endpoint, service, method).await?;

        let request_bytes = encode_json_request(method_descriptor.input(), json_body)
            .map_err(|error| error.to_string())?;
        let path = format!("/{service}/{method}")
            .parse()
            .map_err(|error| format!("invalid gRPC path: {error}"))?;

        let mut request = Request::new(Bytes::from(request_bytes));
        apply_metadata(request.metadata_mut(), &metadata)?;

        let mut grpc = tonic::client::Grpc::new(connect(endpoint).await?);
        grpc.ready()
            .await
            .map_err(|error| format!("gRPC client not ready: {error}"))?;
        let response = grpc
            .unary(request, path, BytesCodec)
            .await
            .map_err(status_to_string)?;
        let response_bytes = response.into_inner();

        let response_json = decode_json_response(method_descriptor.output(), &response_bytes)
            .map_err(|error| error.to_string())?;
        Ok(HttpResponse {
            status: 200,
            status_text: "OK".into(),
            headers: vec![KeyValueField {
                enabled: true,
                name: "content-type".into(),
                value: "application/json".into(),
            }],
            body: ResponseBody::Text(response_json),
            elapsed_ms: 0,
            size_bytes: response_bytes.len(),
        })
    }
    .await;

    timing.prepare_request_ms = prepare_started.elapsed().as_millis();
    match result {
        Ok(mut response) => {
            timing.server_wait_ms = timing.prepare_request_ms;
            response.elapsed_ms = timing.total_ms();
            HttpRequestResult {
                timing,
                response: Ok(response),
            }
        }
        Err(error) => HttpRequestResult {
            timing,
            response: Err(error),
        },
    }
}

async fn resolve_grpc_method(
    endpoint: &str,
    service: &str,
    method: &str,
) -> Result<MethodDescriptor, String> {
    let mut reflection_client = ServerReflectionClient::new(connect(endpoint).await?);
    let file = fetch_file_for_symbol(&mut reflection_client, service)
        .await?
        .ok_or_else(|| format!("service `{service}` was not found via reflection"))?;
    let pool = descriptor_pool_from_file(&file)?;
    pool.get_service_by_name(service)
        .ok_or_else(|| format!("service `{service}` is missing from descriptors"))?
        .methods()
        .find(|candidate| candidate.name() == method)
        .ok_or_else(|| format!("method `{method}` was not found on service `{service}`"))
}

fn message_descriptor_to_template_json(descriptor: MessageDescriptor) -> Result<String, String> {
    if let Some(scalar) = template_for_well_known_scalar(&descriptor) {
        return serde_json::to_string_pretty(&scalar).map_err(|error| error.to_string());
    }

    let value = template_for_message(&descriptor, 0)?;
    serde_json::to_string_pretty(&value).map_err(|error| error.to_string())
}

fn template_for_message(descriptor: &MessageDescriptor, depth: usize) -> Result<JsonValue, String> {
    if depth > 12 {
        return Ok(JsonValue::Object(Map::new()));
    }

    let mut map = Map::new();
    for field in descriptor.fields() {
        map.insert(
            field.json_name().to_string(),
            template_for_field(&field, depth)?,
        );
    }
    Ok(JsonValue::Object(map))
}

fn template_for_field(field: &FieldDescriptor, depth: usize) -> Result<JsonValue, String> {
    if field.is_map() {
        return Ok(JsonValue::Object(Map::new()));
    }
    if field.is_list() {
        return Ok(JsonValue::Array(Vec::new()));
    }
    template_for_kind(&field.kind(), depth)
}

fn template_for_kind(kind: &Kind, depth: usize) -> Result<JsonValue, String> {
    Ok(match kind {
        Kind::Double | Kind::Float => JsonValue::from(0.0),
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 | Kind::Uint32 | Kind::Fixed32 => JsonValue::from(0),
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 | Kind::Uint64 | Kind::Fixed64 => {
            JsonValue::from("0")
        }
        Kind::Bool => JsonValue::from(false),
        Kind::String | Kind::Bytes => JsonValue::from(""),
        Kind::Enum(enum_descriptor) => JsonValue::from(
            enum_descriptor
                .values()
                .next()
                .map(|value| value.name().to_string())
                .unwrap_or_default(),
        ),
        Kind::Message(message) => {
            if let Some(scalar) = template_for_well_known_scalar(message) {
                scalar
            } else if message.is_map_entry() {
                JsonValue::Object(Map::new())
            } else {
                template_for_message(message, depth + 1)?
            }
        }
    })
}

fn template_for_well_known_scalar(descriptor: &MessageDescriptor) -> Option<JsonValue> {
    match descriptor.full_name() {
        "google.protobuf.StringValue" | "google.protobuf.BytesValue" => Some(JsonValue::from("")),
        "google.protobuf.BoolValue" => Some(JsonValue::from(false)),
        "google.protobuf.Int32Value"
        | "google.protobuf.UInt32Value"
        | "google.protobuf.SInt32Value"
        | "google.protobuf.Fixed32Value"
        | "google.protobuf.SFixed32Value" => Some(JsonValue::from(0)),
        "google.protobuf.Int64Value"
        | "google.protobuf.UInt64Value"
        | "google.protobuf.SInt64Value"
        | "google.protobuf.Fixed64Value"
        | "google.protobuf.SFixed64Value" => Some(JsonValue::from("0")),
        "google.protobuf.FloatValue" | "google.protobuf.DoubleValue" => Some(JsonValue::from(0.0)),
        _ => None,
    }
}

async fn connect(endpoint: &str) -> Result<Channel, String> {
    let uri = normalize_endpoint(endpoint)?;
    Channel::from_shared(uri.clone())
        .map_err(|error| error.to_string())?
        .connect()
        .await
        .map_err(|error| format!("failed to connect to {uri}: {error}"))
}

fn normalize_endpoint(endpoint: &str) -> Result<String, String> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Err("gRPC endpoint is empty".into());
    }
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        Ok(endpoint.to_string())
    } else {
        Ok(format!("http://{endpoint}"))
    }
}

async fn list_services(
    client: &mut ServerReflectionClient<Channel>,
) -> Result<Vec<String>, String> {
    let request = ServerReflectionRequest {
        host: String::new(),
        message_request: Some(MessageRequest::ListServices(String::new())),
    };
    let response = next_reflection_message(client, request).await?;
    let MessageResponse::ListServicesResponse(list) = response else {
        return Err("unexpected reflection response while listing services".into());
    };
    Ok(list
        .service
        .into_iter()
        .map(|service| service.name)
        .filter(|name| !name.is_empty())
        .collect())
}

async fn fetch_file_for_symbol(
    client: &mut ServerReflectionClient<Channel>,
    symbol: &str,
) -> Result<Option<FileDescriptorProto>, String> {
    let request = ServerReflectionRequest {
        host: String::new(),
        message_request: Some(MessageRequest::FileContainingSymbol(symbol.to_string())),
    };
    let response = next_reflection_message(client, request).await?;
    let MessageResponse::FileDescriptorResponse(file) = response else {
        return Err("unexpected reflection response while loading descriptors".into());
    };
    let Some(bytes) = file.file_descriptor_proto.into_iter().next() else {
        return Ok(None);
    };
    FileDescriptorProto::decode(bytes.as_slice())
        .map(Some)
        .map_err(|error| format!("failed to decode file descriptor: {error}"))
}

async fn next_reflection_message(
    client: &mut ServerReflectionClient<Channel>,
    request: ServerReflectionRequest,
) -> Result<MessageResponse, String> {
    let mut stream = client
        .server_reflection_info(Request::new(stream::iter(vec![request])))
        .await
        .map_err(status_to_string)?
        .into_inner();

    while let Some(message) = stream.next().await {
        let message = message.map_err(status_to_string)?;
        if let Some(MessageResponse::ErrorResponse(error)) = message.message_response {
            return Err(error.error_message);
        }
        if let Some(response) = message.message_response {
            return Ok(response);
        }
    }

    Err("reflection stream ended without a response".into())
}

fn descriptor_pool_from_file(file: &FileDescriptorProto) -> Result<DescriptorPool, String> {
    let set = FileDescriptorSet {
        file: vec![file.clone()],
    };
    let mut bytes = Vec::new();
    set.encode(&mut bytes)
        .map_err(|error| format!("failed to encode descriptors: {error}"))?;
    DescriptorPool::decode(bytes.as_slice())
        .map_err(|error| format!("failed to build descriptor pool: {error}"))
}

fn methods_for_service(file: &FileDescriptorProto, service_name: &str) -> Vec<GrpcMethodInfo> {
    let package = file.package.as_deref().unwrap_or_default();
    let short_service = service_name
        .strip_prefix(&format!("{package}."))
        .unwrap_or(service_name);

    file.service
        .iter()
        .filter(|service| service.name.as_deref() == Some(short_service))
        .flat_map(|service| methods_from_service(service, package))
        .collect()
}

fn methods_from_service(service: &ServiceDescriptorProto, package: &str) -> Vec<GrpcMethodInfo> {
    let service_name = match service.name.as_deref() {
        Some(name) if package.is_empty() => name.to_string(),
        Some(name) => format!("{package}.{name}"),
        None => return Vec::new(),
    };

    service
        .method
        .iter()
        .filter_map(|method| {
            let method_name = method.name.as_deref()?;
            if method.client_streaming == Some(true) || method.server_streaming == Some(true) {
                return None;
            }
            Some(GrpcMethodInfo {
                service: service_name.clone(),
                method: method_name.to_string(),
            })
        })
        .collect()
}

fn encode_json_request(
    input_descriptor: MessageDescriptor,
    json_body: &str,
) -> Result<Vec<u8>, serde_json::Error> {
    if json_body.trim().is_empty() {
        return Ok(DynamicMessage::new(input_descriptor).encode_to_vec());
    }

    let mut deserializer = serde_json::Deserializer::from_str(json_body);
    let message = input_descriptor.deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(message.encode_to_vec())
}

fn decode_json_response(
    output_descriptor: MessageDescriptor,
    response_bytes: &[u8],
) -> Result<String, serde_json::Error> {
    if response_bytes.is_empty() {
        return Ok(String::new());
    }

    let message = DynamicMessage::decode(output_descriptor, response_bytes)
        .map_err(|error| serde_json::Error::io(std::io::Error::other(error.to_string())))?;
    let mut buf = Vec::new();
    let mut serializer = serde_json::Serializer::pretty(&mut buf);
    message.serialize(&mut serializer)?;
    String::from_utf8(buf).map_err(|error| {
        serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        ))
    })
}

fn apply_metadata(map: &mut MetadataMap, fields: &[KeyValueField]) -> Result<(), String> {
    for field in fields {
        if !field.enabled || field.name.is_empty() {
            continue;
        }
        let key = MetadataKey::from_bytes(field.name.as_bytes())
            .map_err(|error| format!("invalid metadata key `{}`: {error}", field.name))?;
        let value = MetadataValue::try_from(field.value.as_str())
            .map_err(|error| format!("invalid metadata value for `{}`: {error}", field.name))?;
        map.insert(key, value);
    }
    Ok(())
}

fn status_to_string(status: Status) -> String {
    format!("{}: {}", status.code(), status.message())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_json_template_from_descriptor() {
        let pool = DescriptorPool::global();
        let descriptor = pool
            .get_message_by_name("google.protobuf.StringValue")
            .expect("descriptor");
        let json = message_descriptor_to_template_json(descriptor).expect("template");
        assert_eq!(json.trim(), "\"\"");
    }

    #[test]
    fn normalizes_bare_endpoint() {
        assert_eq!(
            normalize_endpoint("localhost:50051").expect("endpoint"),
            "http://localhost:50051"
        );
    }
}
