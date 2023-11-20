# Preservica GraphQL client

## Prerequisites
First install cargo-lambda
```bash
pip3 install cargo-lambda
```

In a different directory, set up a mock secrets manager endpoint.
```bash
mkdir secretsmanager
echo '{"SecretString":"{\"test_user@test.com\": \"password\"}"}' > secretsmanager/get
python -m http.server 2773
```

## Run locally
```bash
export AWS_SESSION_TOKEN=token
export PRESERVICA_URL=http://preservica
cargo lambda watch
```
This will provide an endpoint on `http://localhost:9000`

## Build and deploy
```bash
cargo lambda build --release 
cargo lambda deploy
```