use std::fmt::{Debug};
use config::{Value, Map, ConfigError, Source};
use aws_sdk_ssm;
use aws_sdk_secretsmanager;
use aws_config;
use jmespath;

#[derive(Debug)]
pub struct AwsSource(pub Result<Map<String, Value>, String>);

impl config::Source for AwsSource {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(AwsSource(self.0.clone()))

    }

    fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        match &self.0 {
            Ok(x) => Ok(x.clone()),
            Err(e) => Err(ConfigError::Message(e.to_string())),
        }
    }
}

pub async fn aws_loader() -> Result<Map<String, Value>, String> {
    let mut ssm: Map<String, Vec<String>> = Map::new();
    let mut secret: Map<String, Vec<String>> = Map::new();

    for (key, value) in std::env::vars() {
        if key.starts_with("SSM_") {
            let postfix = key.clone().split_off(4);
            ssm.entry(value).or_insert_with(Vec::new).push(postfix);

        } else if key.starts_with("SECRET_") {
            let postfix = key.clone().split_off(7);
            secret.entry(value).or_insert_with(Vec::new).push(postfix);
        }
    };

    if ssm.len() + secret.len() == 0 {
        return Ok(Map::new());
    }

    let mut r : Map<String, Value> = Map::new();

    let shared_config = aws_config::load_from_env().await;
    let ssm_client = aws_sdk_ssm::Client::new(&shared_config);
    {
        let keys = ssm.iter().map(|(k, _)| k.clone()).collect::<Vec<String>>();
        let resp = ssm_client.get_parameters().set_names(Some(keys)).with_decryption(true).send().await
            .map_err(|e| {
                format!("{:?}", e)
            })?;
        if let Some(pars) = resp.parameters {
            for p in pars {
                let n = p.name.unwrap();
                let v = p.value.unwrap();
                for k in ssm.get(n.as_str()).unwrap() {
                    r.insert(k.to_owned(), Value::from(v.clone()));
                }

            }
        }
    }
    let secret_client = aws_sdk_secretsmanager::Client::new(&shared_config);
    {
        for (n, it) in secret {
            let (name, json_key, stage, id) = parse_secret_identifier(&n)?;
            let resp = secret_client.get_secret_value()
                .secret_id(name)
                .set_version_id(id)
                .set_version_stage(stage)
                .send().await.map_err(|e| {
                format!("{:?}", e)
            })?;
            if let Some(s) = resp.secret_string {
                if json_key.is_none() {
                    for k in it {
                        r.insert(k, Value::from(s.clone()));
                    }
                } else {
                    let expr = jmespath::compile(json_key.as_ref().unwrap()).unwrap();
                    let data = jmespath::Variable::from_json(&s)?;
                    let mut v = expr.search(data).unwrap().to_string();
                    v = v.trim_matches('"').to_string();
                    for k in it {
                        r.insert(k, Value::from(v.clone()));
                    }
                }
            }
        }
    }

    Ok(r)
}

fn parse_secret_identifier(secret_id: &str) -> Result<(String, Option<String>, Option<String>, Option<String>), &'static str> {
    let colon_count = secret_id.matches(':').count();

    // Check if we have a full ARN (at least 6 colons for the basic ARN parts)
    if colon_count >= 6 && secret_id.starts_with("arn:aws:secretsmanager:") {
        // It's a full ARN format
        let parts: Vec<&str> = secret_id.split(':').collect();

        // Validate minimum parts
        if parts.len() < 7 {
            return Err("Invalid ARN format: too few components");
        }

        // Base ARN until secret-name (inclusive)
        let base_arn = format!("{}:{}:{}:{}:{}:{}:{}",
                               parts[0], parts[1], parts[2], parts[3], parts[4], parts[5], parts[6]);

        // Optional parts
        let json_key = if parts.len() > 7 { Some(parts[7].to_string()) } else { None };
        let version_stage = if parts.len() > 8 { Some(parts[8].to_string()) } else { None };
        let version_id = if parts.len() > 9 { Some(parts[9].to_string()) } else { None };

        Ok((base_arn, json_key, version_stage, version_id))
    } else {
        // It's just the secret name part format: secret-name:[json-key]:[version-stage]:[version-id]
        let parts: Vec<&str> = secret_id.split(':').collect();

        // The first part is always the secret name
        let secret_name = parts[0].to_string();

        // Optional parts
        let json_key = if parts.len() > 1 { Some(parts[1].to_string()) } else { None };
        let version_stage = if parts.len() > 2 { Some(parts[2].to_string()) } else { None };
        let version_id = if parts.len() > 3 { Some(parts[3].to_string()) } else { None };

        Ok((secret_name, json_key, version_stage, version_id))
    }
}