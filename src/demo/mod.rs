use crate::domain::{
    BodyType, Collection, CollectionFolder, Environment, HttpMethod, KeyValueField, Request,
    Variable, Workspace,
};

pub fn demo_workspaces() -> Vec<Workspace> {
    let mut environment = Environment::new("JSONPlaceholder");
    environment.variables = vec![
        Variable::from_strings(
            "baseUrl".into(),
            "https://jsonplaceholder.typicode.com".into(),
        ),
        Variable::from_strings("apiKey".into(), "demo-key".into()),
    ];

    vec![Workspace {
        name: "Personal".into(),
        variables: vec![Variable::from_strings("apiVersion".into(), "v1".into())],
        environments: vec![environment],
        collections: vec![demo_api_collection()],
    }]
}

fn demo_api_collection() -> Collection {
    let mut collection = Collection::new("JSONPlaceholder");
    collection.variables = vec![
        Variable::from_strings("postId".into(), "1".into()),
        Variable::from_strings("userId".into(), "1".into()),
    ];
    collection.requests = vec![demo_get("Get Users", "{{baseUrl}}/users")];
    collection.folders = vec![
        posts_folder(),
        users_folder(),
        comments_folder(),
        todos_folder(),
        albums_folder(),
    ];
    collection
}

fn posts_folder() -> CollectionFolder {
    let mut folder = CollectionFolder::new("Posts");
    folder.requests = vec![
        demo_get("List Posts", "{{baseUrl}}/posts"),
        demo_get("Get Post", "{{baseUrl}}/posts/{{postId}}"),
        demo_request("Posts by User", |request| {
            request.url = "{{baseUrl}}/posts".into();
            request.query_params = vec![
                KeyValueField {
                    enabled: true,
                    name: "userId".into(),
                    value: "{{userId}}".into(),
                },
                KeyValueField::empty(),
            ];
        }),
        demo_get(
            "Post Comments",
            "{{baseUrl}}/posts/{{postId}}/comments",
        ),
        demo_request("Create Post", |request| {
            request.method = HttpMethod::Post;
            request.url = "{{baseUrl}}/posts".into();
            request.headers = vec![
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
            ];
            request.body_type = BodyType::Json;
            request.body = r#"{
  "title": "{{title}}",
  "body": "Created from api-helper demo",
  "userId": {{userId}}
}"#
            .into();
            request.variables = vec![
                Variable::from_strings("title".into(), "Hello from variables".into()),
                Variable::empty(),
            ];
            request.pre_request_script = r#"host.setVar("requestId", String(Date.now()));
console.log("Sending request to", host.getEnvVar("baseUrl"));"#
                .into();
        }),
        demo_request("Update Post", |request| {
            request.method = HttpMethod::Put;
            request.url = "{{baseUrl}}/posts/{{postId}}".into();
            request.headers = vec![
                KeyValueField {
                    enabled: true,
                    name: "Content-Type".into(),
                    value: "application/json".into(),
                },
                KeyValueField::empty(),
            ];
            request.body_type = BodyType::Json;
            request.body = r#"{
  "id": {{postId}},
  "title": "Updated title",
  "body": "Updated body",
  "userId": {{userId}}
}"#
            .into();
        }),
        demo_request("Patch Post", |request| {
            request.method = HttpMethod::Patch;
            request.url = "{{baseUrl}}/posts/{{postId}}".into();
            request.headers = vec![
                KeyValueField {
                    enabled: true,
                    name: "Content-Type".into(),
                    value: "application/json".into(),
                },
                KeyValueField::empty(),
            ];
            request.body_type = BodyType::Json;
            request.body = r#"{
  "title": "Patched title"
}"#
            .into();
        }),
        demo_request("Delete Post", |request| {
            request.method = HttpMethod::Delete;
            request.url = "{{baseUrl}}/posts/{{postId}}".into();
        }),
    ];
    folder
}

fn users_folder() -> CollectionFolder {
    let mut folder = CollectionFolder::new("Users");
    folder.expanded = false;
    folder.requests = vec![
        demo_get("Get User", "{{baseUrl}}/users/{{userId}}"),
        demo_get("User Posts", "{{baseUrl}}/users/{{userId}}/posts"),
        demo_get("User Albums", "{{baseUrl}}/users/{{userId}}/albums"),
        demo_get("User Todos", "{{baseUrl}}/users/{{userId}}/todos"),
        demo_request("Get User → save session", |request| {
            request.url = "{{baseUrl}}/users/{{userId}}".into();
            request.post_response_script = r#"host.setVar("activeUserId", String(res.body.id));
host.setVar("activeUserName", res.body.name);
host.setVar("sessionToken", "tok-" + String(Date.now()));
console.log("Saved session for", res.body.name, "from", res.getUrl());"#
                .into();
        }),
        demo_request("User Posts (from session)", |request| {
            request.url = "{{baseUrl}}/users/{{activeUserId}}/posts".into();
            request.headers = vec![
                KeyValueField {
                    enabled: true,
                    name: "X-Session-Token".into(),
                    value: "{{sessionToken}}".into(),
                },
                KeyValueField::empty(),
            ];
        }),
    ];
    folder
}

fn comments_folder() -> CollectionFolder {
    let mut folder = CollectionFolder::new("Comments");
    folder.expanded = false;
    folder.requests = vec![
        demo_get("List Comments", "{{baseUrl}}/comments"),
        demo_request("Comments for Post", |request| {
            request.url = "{{baseUrl}}/comments".into();
            request.query_params = vec![
                KeyValueField {
                    enabled: true,
                    name: "postId".into(),
                    value: "{{postId}}".into(),
                },
                KeyValueField::empty(),
            ];
        }),
    ];
    folder
}

fn todos_folder() -> CollectionFolder {
    let mut folder = CollectionFolder::new("Todos");
    folder.expanded = false;
    folder.variables = vec![Variable::from_strings("todoId".into(), "1".into())];
    folder.requests = vec![
        demo_get("List Todos", "{{baseUrl}}/todos"),
        demo_get("Get Todo", "{{baseUrl}}/todos/{{todoId}}"),
        demo_request("Complete Todo", |request| {
            request.method = HttpMethod::Patch;
            request.url = "{{baseUrl}}/todos/{{todoId}}".into();
            request.headers = vec![
                KeyValueField {
                    enabled: true,
                    name: "Content-Type".into(),
                    value: "application/json".into(),
                },
                KeyValueField::empty(),
            ];
            request.body_type = BodyType::Json;
            request.body = r#"{
  "completed": true
}"#
            .into();
        }),
    ];
    folder
}

fn albums_folder() -> CollectionFolder {
    let mut folder = CollectionFolder::new("Albums");
    folder.expanded = false;
    folder.variables = vec![Variable::from_strings("albumId".into(), "1".into())];
    folder.requests = vec![
        demo_get("List Albums", "{{baseUrl}}/albums"),
        demo_get("Get Album", "{{baseUrl}}/albums/{{albumId}}"),
        demo_get(
            "Album Photos",
            "{{baseUrl}}/albums/{{albumId}}/photos",
        ),
        demo_get("List Photos", "{{baseUrl}}/photos"),
    ];
    folder
}

fn demo_request(name: &str, configure: impl FnOnce(&mut Request)) -> Request {
    let mut request = Request::new(name);
    configure(&mut request);
    request
}

fn demo_get(name: &str, url: &str) -> Request {
    let mut request = Request::new(name);
    request.url = url.into();
    request
}
