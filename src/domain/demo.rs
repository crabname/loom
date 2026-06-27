use super::{
    default_form_fields, default_key_value_fields, default_multipart_fields, BodyType,
    Collection, HttpMethod, KeyValueField, MultipartField, MultipartFieldType, Request,
    RequestProtocol,
};

pub fn demo_collections() -> Vec<Collection> {
    vec![
        Collection {
            name: "Demo API".into(),
            expanded: true,
            requests: vec![
                Request {
                    name: "Get Users".into(),
                    protocol: RequestProtocol::Http,
                    method: HttpMethod::Get,
                    url: "https://jsonplaceholder.typicode.com/users".into(),
                    query_params: default_key_value_fields(),
                    headers: default_key_value_fields(),
                    body_type: BodyType::None,
                    body: String::new(),
                    form_fields: default_form_fields(),
                    multipart_fields: default_multipart_fields(),
                },
                Request {
                    name: "Get User".into(),
                    protocol: RequestProtocol::Http,
                    method: HttpMethod::Get,
                    url: "https://jsonplaceholder.typicode.com/users/1".into(),
                    query_params: vec![
                        KeyValueField {
                            enabled: true,
                            name: "_expand".into(),
                            value: "posts".into(),
                        },
                        KeyValueField::empty(),
                    ],
                    headers: default_key_value_fields(),
                    body_type: BodyType::None,
                    body: String::new(),
                    form_fields: default_form_fields(),
                    multipart_fields: default_multipart_fields(),
                },
                Request {
                    name: "Create Post".into(),
                    protocol: RequestProtocol::Http,
                    method: HttpMethod::Post,
                    url: "https://jsonplaceholder.typicode.com/posts".into(),
                    query_params: default_key_value_fields(),
                    headers: vec![
                        KeyValueField {
                            enabled: true,
                            name: "Accept".into(),
                            value: "application/json".into(),
                        },
                        KeyValueField::empty(),
                    ],
                    body_type: BodyType::Json,
                    body: r#"{
  "title": "Hello",
  "body": "World",
  "userId": 1
}"#
                    .into(),
                    form_fields: default_form_fields(),
                    multipart_fields: default_multipart_fields(),
                },
            ],
        },
        Collection {
            name: "Local".into(),
            expanded: true,
            requests: vec![
                Request {
                    name: "Health Check".into(),
                    protocol: RequestProtocol::Http,
                    method: HttpMethod::Get,
                    url: "http://localhost:8080/health".into(),
                    query_params: default_key_value_fields(),
                    headers: default_key_value_fields(),
                    body_type: BodyType::None,
                    body: String::new(),
                    form_fields: default_form_fields(),
                    multipart_fields: default_multipart_fields(),
                },
                Request {
                    name: "Parse Board".into(),
                    protocol: RequestProtocol::Http,
                    method: HttpMethod::Post,
                    url: "http://localhost:8080/parse-board".into(),
                    query_params: default_key_value_fields(),
                    headers: default_key_value_fields(),
                    body_type: BodyType::Multipart,
                    body: String::new(),
                    form_fields: default_form_fields(),
                    multipart_fields: vec![
                        MultipartField {
                            enabled: true,
                            name: "image".into(),
                            value: "/Users/anrey/Desktop/Снимок экрана\u{00a0}— 2026-06-27 в\u{00a0}20.50.16.png".into(),
                            field_type: MultipartFieldType::File,
                            content_type: "image/png".into(),
                        },
                        MultipartField::empty(),
                    ],
                },
                Request {
                    name: "List Services".into(),
                    protocol: RequestProtocol::Grpc,
                    method: HttpMethod::Post,
                    url: "grpc://localhost:50051".into(),
                    query_params: default_key_value_fields(),
                    headers: default_key_value_fields(),
                    body_type: BodyType::None,
                    body: String::new(),
                    form_fields: default_form_fields(),
                    multipart_fields: default_multipart_fields(),
                },
            ],
        },
    ]
}
