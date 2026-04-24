#[cfg(feature = "aws")]
mod aws_loader;

use config::builder::{AsyncState, DefaultState};
use config::ConfigBuilder;
use serde::Deserialize;
use std::fmt::Debug;
use std::path::PathBuf;

fn file_path_from_env(name: Option<&str>) -> Option<String> {
    if let Some(name) = name {
        Some(std::env::var(name).expect("specified env var not found"))
    } else {
        std::env::var("CONFIG").ok()
    }
}

fn base_sync_builder() -> ConfigBuilder<DefaultState> {
    let mut builder = ConfigBuilder::<DefaultState>::default();
    builder = builder.add_source(config::Environment::default());
    builder
}

fn base_async_builder() -> ConfigBuilder<AsyncState> {
    let mut builder = ConfigBuilder::<AsyncState>::default();
    builder = builder.add_source(config::Environment::default());
    builder
}

/// Parse configuration from environment variables and an optional file path.
///
/// If `name` is `Some`, uses that env var for the config file path.
/// If `None`, falls back to `CONFIG` when set.
pub fn parse<T>(name: Option<&str>) -> T
where T: for<'de> Deserialize<'de> + Clone + Debug {
    let mut builder = base_sync_builder();
    if let Some(path) = file_path_from_env(name) {
        builder = builder.add_source(config::File::with_name(&path));
    }

    let config = builder.build().expect("failed to parse config");
    let parsed: T = config.try_deserialize().unwrap();
    parsed
}

/// Parse configuration from environment variables and an optional file path.
///
/// If `path` is `Some`, uses it as the config file path.
/// If `None`, falls back to `CONFIG` when set.
pub fn parse_file<T>(path: Option<impl Into<PathBuf>>) -> T
where
    T: for<'de> Deserialize<'de> + Clone + Debug,
{
    let mut builder = base_sync_builder();
    let path = match path {
        Some(path) => Some(path.into().to_string_lossy().into_owned()),
        None => file_path_from_env(None),
    };
    if let Some(path) = path {
        builder = builder.add_source(config::File::with_name(&path));
    }

    let config = builder.build().expect("failed to parse config");
    let parsed: T = config.try_deserialize().unwrap();
    parsed
}

/// Async variant of `parse`.
///
/// If `name` is `Some`, uses that env var for the config file path.
/// If `None`, falls back to `CONFIG` when set.
pub async fn parse_async<T>(name: Option<&str>) -> T
where T: for<'de> Deserialize<'de> + Clone + Debug {
    let mut builder = base_async_builder();
    if let Some(path) = file_path_from_env(name) {
        builder = builder.add_source(config::File::with_name(&path));
    }
    #[cfg(feature = "aws")]
    {
        let aws_m = aws_loader::AwsSource(aws_loader::aws_loader().await);
        builder = builder.add_source(aws_m);
    }

    let config = builder.build().await.expect("failed to parse config");
    let parsed: T = config.try_deserialize().unwrap();
    parsed
}

/// Async variant of `parse_file`.
///
/// If `path` is `Some`, uses it as the config file path.
/// If `None`, falls back to `CONFIG` when set.
pub async fn parse_async_file<T>(path: Option<impl Into<PathBuf>>) -> T
where
    T: for<'de> Deserialize<'de> + Clone + Debug,
{
    let mut builder = base_async_builder();
    let path = match path {
        Some(path) => Some(path.into().to_string_lossy().into_owned()),
        None => file_path_from_env(None),
    };
    if let Some(path) = path {
        builder = builder.add_source(config::File::with_name(&path));
    }
    #[cfg(feature = "aws")]
    {
        let aws_m = aws_loader::AwsSource(aws_loader::aws_loader().await);
        builder = builder.add_source(aws_m);
    }

    let config = builder.build().await.expect("failed to parse config");
    let parsed: T = config.try_deserialize().unwrap();
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize, Debug, Clone)]
    pub struct Conf {
        pub port: u16,
        pub host: String,
    }

    #[test]
    fn work_from_env() {
        unsafe {
            std::env::set_var("port", "8080");
            std::env::set_var("host", "foo.bar");
        }
        let p = parse::<Conf>(None);
        println!("{:?}", p);
        assert_eq!(p.port, 8080);
        assert_eq!(p.host, "foo.bar".to_string());
        unsafe {
            std::env::remove_var("port");
            std::env::remove_var("host");
        }
    }

    #[test]
    #[should_panic]
    fn panic_no_env() {
        let _p = parse::<Conf>(None);
    }

    #[test]
    fn work_from_default_file() {
        unsafe {
            std::env::set_var("CONFIG", "env.sample.toml");
        }
        let p = parse::<Conf>(None);
        println!("{:?}", p);
        assert_eq!(p.port, 8080);
        assert_eq!(p.host, "foo.bar".to_string());
        unsafe {
            std::env::remove_var("CONFIG");
        }
    }

    #[test]
    #[should_panic]
    fn panic_with_non_exist_file() {
        struct Cleanup;
        impl Drop for Cleanup {
            fn drop(&mut self) {
                unsafe {
                    std::env::remove_var("CONFIG");
                }
            }
        }
        unsafe {
            std::env::set_var("CONFIG", "bar.toml");
        }
        let _cleanup = Cleanup;
        let _p = parse::<Conf>(None);
    }


    #[test]
    fn duplicate_var_in_env_and_file() {
        unsafe {
            std::env::set_var("port", "8099");
            std::env::set_var("host", "foo.bar");
            std::env::set_var("CONFIG", "env.sample.toml");
        }
        let p = parse::<Conf>(None);
        println!("{:?}", p);
        assert_eq!(p.port, 8080);
        assert_eq!(p.host, "foo.bar".to_string());
        unsafe {
            std::env::remove_var("port");
            std::env::remove_var("host");
            std::env::remove_var("CONFIG");
        }
    }

    #[tokio::test]
    async fn aws() {
        #[derive(Deserialize, Debug, Clone)]
        pub struct Conf {
            server: Server,
            db: Db,
        }

        #[derive(Deserialize, Debug, Clone)]
        pub struct Server {
            pub port: u16,
            pub host: String,
        }

        #[derive(Deserialize, Debug, Clone)]
        pub struct Db {
            pub user: String,
            pub password: String,
        }

        unsafe {
            std::env::set_var("server.port", "8080");
            std::env::set_var("SSM_server.host", "/env/host");
            std::env::set_var("SECRET_db.user", "dummy:username::");
            std::env::set_var("SECRET_db.password", "dummy:password::");
        }
        let p = parse_async::<Conf>(None).await;
        println!("{:?}", p);
        assert_eq!(p.server.port, 8080);
        assert_eq!(p.server.host, "aws.local".to_string());
        assert_eq!(p.db.user, "dummy".to_string());
        unsafe {
            std::env::remove_var("server.port");
            std::env::remove_var("SSM_server.host");
            std::env::remove_var("SECRET_db.user");
            std::env::remove_var("SECRET_db.password");
        }
    }

    #[test]
    fn embedded_struct() {
        #[derive(Deserialize, Debug, Clone)]
        pub struct Conf {
            server: Server,
            db: Db,
        }

        #[derive(Deserialize, Debug, Clone)]
        pub struct Server {
            pub port: u16,
            pub host: String,
        }

        #[derive(Deserialize, Debug, Clone)]
        pub struct Db {
            pub user: u16,
            pub password: String,
        }
    }

}
