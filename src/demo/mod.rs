use crate::domain::{
    default_form_fields, default_key_value_fields, default_multipart_fields, default_variables,
    BodyType, Collection, CollectionFolder, Environment, HttpMethod, KeyValueField, Request,
    RequestProtocol, Variable, Workspace,
};

pub fn demo_workspaces() -> Vec<Workspace> {
    vec![Workspace {
        name: "Personal".into(),
        variables: vec![Variable::from_strings("apiVersion".into(), "v1".into())],
        environments: vec![Environment {
            name: "JSONPlaceholder".into(),
            variables: vec![
                Variable::from_strings(
                    "baseUrl".into(),
                    "https://jsonplaceholder.typicode.com".into(),
                ),
                Variable::from_strings("apiKey".into(), "demo-key".into()),
            ],
        }],
        collections: vec![demo_api_collection()],
    }]
}

fn demo_api_collection() -> Collection {
    Collection {
        name: "JSONPlaceholder".into(),
        expanded: true,
        variables: vec![
            Variable::from_strings("postId".into(), "1".into()),
            Variable::from_strings("userId".into(), "1".into()),
        ],
        environments: vec![],
        requests: vec![demo_get("Get Users", "{{baseUrl}}/users")],
        folders: vec![
            posts_folder(),
            users_folder(),
            comments_folder(),
            todos_folder(),
            albums_folder(),
        ],
    }
}

fn posts_folder() -> CollectionFolder {
    CollectionFolder {
        name: "Posts".into(),
        expanded: true,
        variables: default_variables(),
        requests: vec![
            demo_get("List Posts", "{{baseUrl}}/posts"),
            demo_get("Get Post", "{{baseUrl}}/posts/{{postId}}"),
            Request {
                name: "Posts by User".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Get,
                url: "{{baseUrl}}/posts".into(),
                query_params: vec![
                    KeyValueField {
                        enabled: true,
                        name: "userId".into(),
                        value: "{{userId}}".into(),
                    },
                    KeyValueField::empty(),
                ],
                headers: default_key_value_fields(),
                body_type: BodyType::None,
                body: String::new(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: default_variables(),
                pre_request_script: String::new(),
                post_response_script: String::new(),
            },
            demo_get(
                "Post Comments",
                "{{baseUrl}}/posts/{{postId}}/comments",
            ),
            Request {
                name: "Create Post".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Post,
                url: "{{baseUrl}}/posts".into(),
                query_params: default_key_value_fields(),
                headers: vec![
                    KeyValueField {
                        enabled: true,
                        name: "Content-Type".into(),
                        value: "application/json".into(),
                    },
                    KeyValueField {
                        enabled: true,
                        name: "Authorization".into(),
                        value: "Bearer {{apiKey}}".into(),
                    },
                    KeyValueField {
                        enabled: true,
                        name: "X-Api-Version".into(),
                        value: "{{apiVersion}}".into(),
                    },
                    KeyValueField {
                        enabled: true,
                        name: "X-Request-Id".into(),
                        value: "{{requestId}}".into(),
                    },
                    KeyValueField::empty(),
                ],
                body_type: BodyType::Json,
                body: r#"{
  "title": "{{title}}",
  "body": "Created from api-helper demo",
  "userId": {{userId}}
}"#
                .into(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: vec![
                    Variable::from_strings("title".into(), "Hello from variables".into()),
                    Variable::empty(),
                ],
                pre_request_script: r#"host.setVar("requestId", String(Date.now()));
console.log("Sending request to", host.getEnvVar("baseUrl"));"#
                    .into(),
                post_response_script: String::new(),
            },
            Request {
                name: "Update Post".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Put,
                url: "{{baseUrl}}/posts/{{postId}}".into(),
                query_params: default_key_value_fields(),
                headers: vec![
                    KeyValueField {
                        enabled: true,
                        name: "Content-Type".into(),
                        value: "application/json".into(),
                    },
                    KeyValueField::empty(),
                ],
                body_type: BodyType::Json,
                body: r#"{
  "id": {{postId}},
  "title": "Updated title",
  "body": "Updated body",
  "userId": {{userId}}
}"#
                .into(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: default_variables(),
                pre_request_script: String::new(),
                post_response_script: String::new(),
            },
            Request {
                name: "Patch Post".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Patch,
                url: "{{baseUrl}}/posts/{{postId}}".into(),
                query_params: default_key_value_fields(),
                headers: vec![
                    KeyValueField {
                        enabled: true,
                        name: "Content-Type".into(),
                        value: "application/json".into(),
                    },
                    KeyValueField::empty(),
                ],
                body_type: BodyType::Json,
                body: r#"{
  "title": "Patched title"
}"#
                .into(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: default_variables(),
                pre_request_script: String::new(),
                post_response_script: String::new(),
            },
            Request {
                name: "Delete Post".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Delete,
                url: "{{baseUrl}}/posts/{{postId}}".into(),
                query_params: default_key_value_fields(),
                headers: default_key_value_fields(),
                body_type: BodyType::None,
                body: String::new(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: default_variables(),
                pre_request_script: String::new(),
                post_response_script: String::new(),
            },
        ],
    }
}

fn users_folder() -> CollectionFolder {
    CollectionFolder {
        name: "Users".into(),
        expanded: false,
        variables: default_variables(),
        requests: vec![
            demo_get("Get User", "{{baseUrl}}/users/{{userId}}"),
            demo_get("User Posts", "{{baseUrl}}/users/{{userId}}/posts"),
            demo_get("User Albums", "{{baseUrl}}/users/{{userId}}/albums"),
            demo_get("User Todos", "{{baseUrl}}/users/{{userId}}/todos"),
            Request {
                name: "Get User → save session".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Get,
                url: "{{baseUrl}}/users/{{userId}}".into(),
                query_params: default_key_value_fields(),
                headers: default_key_value_fields(),
                body_type: BodyType::None,
                body: String::new(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: default_variables(),
                pre_request_script: String::new(),
                post_response_script: r#"host.setVar("activeUserId", String(res.body.id));
host.setVar("activeUserName", res.body.name);
host.setVar("sessionToken", "tok-" + String(Date.now()));
console.log("Saved session for", res.body.name, "from", res.getUrl());"#
                    .into(),
            },
            Request {
                name: "User Posts (from session)".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Get,
                url: "{{baseUrl}}/users/{{activeUserId}}/posts".into(),
                query_params: default_key_value_fields(),
                headers: vec![
                    KeyValueField {
                        enabled: true,
                        name: "X-Session-Token".into(),
                        value: "{{sessionToken}}".into(),
                    },
                    KeyValueField::empty(),
                ],
                body_type: BodyType::None,
                body: String::new(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: default_variables(),
                pre_request_script: String::new(),
                post_response_script: String::new(),
            },
        ],
    }
}

fn comments_folder() -> CollectionFolder {
    CollectionFolder {
        name: "Comments".into(),
        expanded: false,
        variables: default_variables(),
        requests: vec![
            demo_get("List Comments", "{{baseUrl}}/comments"),
            Request {
                name: "Comments for Post".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Get,
                url: "{{baseUrl}}/comments".into(),
                query_params: vec![
                    KeyValueField {
                        enabled: true,
                        name: "postId".into(),
                        value: "{{postId}}".into(),
                    },
                    KeyValueField::empty(),
                ],
                headers: default_key_value_fields(),
                body_type: BodyType::None,
                body: String::new(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: default_variables(),
                pre_request_script: String::new(),
                post_response_script: String::new(),
            },
        ],
    }
}

fn todos_folder() -> CollectionFolder {
    CollectionFolder {
        name: "Todos".into(),
        expanded: false,
        variables: vec![Variable::from_strings("todoId".into(), "1".into())],
        requests: vec![
            demo_get("List Todos", "{{baseUrl}}/todos"),
            demo_get("Get Todo", "{{baseUrl}}/todos/{{todoId}}"),
            Request {
                name: "Complete Todo".into(),
                protocol: RequestProtocol::Http,
                method: HttpMethod::Patch,
                url: "{{baseUrl}}/todos/{{todoId}}".into(),
                query_params: default_key_value_fields(),
                headers: vec![
                    KeyValueField {
                        enabled: true,
                        name: "Content-Type".into(),
                        value: "application/json".into(),
                    },
                    KeyValueField::empty(),
                ],
                body_type: BodyType::Json,
                body: r#"{
  "completed": true
}"#
                .into(),
                form_fields: default_form_fields(),
                multipart_fields: default_multipart_fields(),
                variables: default_variables(),
                pre_request_script: String::new(),
                post_response_script: String::new(),
            },
        ],
    }
}

fn albums_folder() -> CollectionFolder {
    CollectionFolder {
        name: "Albums".into(),
        expanded: false,
        variables: vec![Variable::from_strings("albumId".into(), "1".into())],
        requests: vec![
            demo_get("List Albums", "{{baseUrl}}/albums"),
            demo_get("Get Album", "{{baseUrl}}/albums/{{albumId}}"),
            demo_get(
                "Album Photos",
                "{{baseUrl}}/albums/{{albumId}}/photos",
            ),
            demo_get("List Photos", "{{baseUrl}}/photos"),
        ],
    }
}

fn demo_get(name: &str, url: &str) -> Request {
    Request {
        name: name.into(),
        protocol: RequestProtocol::Http,
        method: HttpMethod::Get,
        url: url.into(),
        query_params: default_key_value_fields(),
        headers: default_key_value_fields(),
        body_type: BodyType::None,
        body: String::new(),
        form_fields: default_form_fields(),
        multipart_fields: default_multipart_fields(),
        variables: default_variables(),
        pre_request_script: String::new(),
        post_response_script: String::new(),
    }
}
