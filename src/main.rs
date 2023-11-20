use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldResult, GraphQLObject, Variables,
};
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde_json::Value;
use uuid::Uuid;
struct Query;
use reqwest::Client;
use serde::Deserialize;
use std::env;
struct Context;

impl juniper::Context for Context {}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InformationObject {
    #[serde(rename = "Ref")]
    reference: Uuid,
    title: String,
    security_tag: String,
    parent: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EntityResponse {
    information_object: InformationObject,
}

#[derive(GraphQLObject)]
#[graphql(description = "A Preservica entity")]
struct Entity {
    reference: Uuid,
    title: String,
    security_tag: String,
    parent: Uuid,
}
#[graphql_object(
context = Context,
)]
impl Query {
    async fn entity(&self, reference: Uuid) -> FieldResult<Entity> {
        let token = get_token().await?;
        let res = get_entity(reference, token).await?;
        let entity_response: EntityResponse = quick_xml::de::from_str(&res)?;
        let information_object = entity_response.information_object;
        Ok(Entity {
            reference: information_object.reference,
            title: information_object.title,
            security_tag: information_object.security_tag,
            parent: information_object.parent,
        })
    }
}

type Schema = juniper::RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

struct Credentials {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct SecretsManagerResponse {
    #[serde(rename = "SecretString")]
    secret_string: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
}

fn preservica_url() -> String {
    env::var("PRESERVICA_URL").expect("Missing Preservica URL")
}

async fn get_token() -> Result<String, Error> {
    let credentials = get_credentials().await?;
    let preservica_url = preservica_url();
    let params = [
        ("username", credentials.username),
        ("password", credentials.password),
    ];
    Client::new()
        .post(format!("{preservica_url}/api/accesstoken/login"))
        .form(&params)
        .send()
        .await?
        .json::<TokenResponse>()
        .await
        .map(|res| res.token)
        .map_err(|e| Error::from(e.to_string()))
}

async fn get_entity(reference: Uuid, token: String) -> Result<String, reqwest::Error> {
    let preservica_url = preservica_url();
    let url = format!("{preservica_url}/api/entity/information-objects/{reference}");
    Client::new()
        .get(url)
        .header("Preservica-Access-Token", token)
        .send()
        .await?
        .text()
        .await
}

async fn get_credentials() -> Result<Credentials, Error> {
    let session_token = env::var("AWS_SESSION_TOKEN").expect("Missing session token");
    let resp = Client::new().get("http://localhost:2773/secretsmanager/get?secretId=sandbox-preservica-6-preservicav6login")
        .header("X-Aws-Parameters-Secrets-Token", session_token)
        .send()
        .await?
        .json::<SecretsManagerResponse>()
        .await?;
    let secret_string = resp.secret_string;
    let secret_json: Value = serde_json::from_str(&secret_string).expect("Invalid json");
    let obj = &secret_json
        .as_object()
        .ok_or("Error converting secret json to object")?;
    let username = obj.keys().last().ok_or("Empty keys list")?;
    let password = obj
        .get(username)
        .and_then(|pwd| pwd.as_str())
        .ok_or("Cannot retrieve password")?;
    Ok(Credentials {
        username: username.to_owned(),
        password: password.to_owned(),
    })
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let body = String::from_utf8(event.body().to_vec())?;
    let json: Value = serde_json::from_str(&body)?;
    let query = json["query"].as_str().ok_or("Missing query")?;
    let ctx = Context {};
    let (res, _errors) = juniper::execute(
        query,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &ctx,
    )
    .await
    .unwrap();

    let response_str = res.to_string();

    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(response_str.into())
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
