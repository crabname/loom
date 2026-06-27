use super::{
    default_form_fields, default_key_value_fields, default_multipart_fields, default_variables,
    BodyType, Collection, Environment, HttpMethod, KeyValueField, MultipartField,
    MultipartFieldType, Request, RequestProtocol, Variable, Workspace,
};

pub fn demo_workspaces() -> Vec<Workspace> {
    vec![
        Workspace {
            name: "Personal".into(),
            variables: vec![Variable::from_strings(
                "apiVersion".into(),
                "v1".into(),
            )],
            environments: vec![
                Environment {
                    name: "Production".into(),
                    variables: vec![
                        Variable::from_strings("baseUrl".into(), "https://api.example.com".into()),
                        Variable::from_strings("apiKey".into(), "prod-key".into()),
                    ],
                },
                Environment {
                    name: "Staging".into(),
                    variables: vec![
                        Variable::from_strings("baseUrl".into(), "https://staging.example.com".into()),
                        Variable::from_strings("apiKey".into(), "staging-key".into()),
                    ],
                },
            ],
            collections: vec![demo_api_collection()],
        },
        Workspace {
            name: "Local Dev".into(),
            variables: vec![Variable::from_strings(
                "apiVersion".into(),
                "dev".into(),
            )],
            environments: vec![Environment {
                name: "Local".into(),
                variables: vec![Variable::from_strings(
                    "baseUrl".into(),
                    "http://localhost:8080".into(),
                )],
            }],
            collections: vec![local_collection()],
        },
    ]
}

fn demo_api_collection() -> Collection {
    Collection {
        name: "Demo API".into(),
        expanded: true,
        variables: vec![Variable::from_strings(
            "resource".into(),
            "users".into(),
        )],
        environments: vec![Environment {
            name: "JSONPlaceholder".into(),
            variables: vec![Variable::from_strings(
                "baseUrl".into(),
                "https://jsonplaceholder.typicode.com".into(),
            )],
        }],
        requests: vec![
            Request {
                name: "Get Users".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Get,
                url: "{{baseUrl}}/{{resource}}".into(),
                query_params: default_key_value_fields(),
                headers: default_key_value_fields(),
                body_type: BodyType::None,
                body: String::new(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: default_variables(),
            },
        ],
        folders: vec![super::CollectionFolder {
            name: "User endpoints".into(),
            expanded: true,
            variables: vec![Variable::from_strings(
                "userId".into(),
                "1".into(),
            )],
            requests: vec![
            Request {
                name: "Get User".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Get,
                url: "{{baseUrl}}/users/{{userId}}".into(),
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
                variables: default_variables(),
            },
            Request {
                name: "Create Post".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Post,
                url: "{{baseUrl}}/posts".into(),
                query_params: default_key_value_fields(),
                headers: vec![
                    KeyValueField {
                        enabled: true,
                        name: "Accept".into(),
                        value: "application/json".into(),
                    },
                    KeyValueField {
                        enabled: true,
                        name: "X-Api-Version".into(),
                        value: "{{apiVersion}}".into(),
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
                variables: default_variables(),
            },
            ],
        }],
    }
}

fn local_collection() -> Collection {
    Collection {
        name: "Local".into(),
        expanded: true,
        variables: default_variables(),
        environments: vec![Environment {
            name: "Localhost".into(),
            variables: vec![Variable::from_strings(
                "baseUrl".into(),
                "http://localhost:8080".into(),
            )],
        }],
        folders: vec![],
        requests: vec![
            Request {
                name: "Health Check".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Get,
                url: "{{baseUrl}}/health".into(),
                query_params: default_key_value_fields(),
                headers: default_key_value_fields(),
                body_type: BodyType::None,
                body: String::new(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: default_variables(),
            },
            Request {
                name: "Parse Board".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Post,
                url: "{{baseUrl}}/parse-board".into(),
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
                variables: default_variables(),
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
                variables: default_variables(),
            },
        ],
    }
}
