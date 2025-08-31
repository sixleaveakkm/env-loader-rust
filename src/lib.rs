use config::Config;
use std::fmt::Debug;
use serde::Deserialize;

pub fn parse<T>(name: Option<&str>) -> T
where T: for<'de> Deserialize<'de>
+ Clone + Debug {
    let mut builder = Config::builder();
    if let Some(name) = name {
        let path = std::env::var(name).ok()
            .expect("specified env var not found");
        builder = builder.add_source(config::File::with_name(path.as_str()));
    } else {
        let path = std::env::var("CONFIG").ok();
        if let Some(path) = path {
            builder = builder.add_source(config::File::with_name(path.as_str()));
        }
    }
    builder = builder.add_source(config::Environment::default());

    let config = builder.build().expect("failed to parse config");
    let parsed: T = config.try_deserialize().unwrap();
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::*;

    #[derive(Deserialize, Debug, Clone)]
    pub struct Conf {
        pub port: u16,
        pub host: String,
    }

    #[test]
    #[serial]
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
    #[serial]
    #[should_panic]
    fn panic_no_env() {
        let p = parse::<Conf>(None);
    }

    #[test]
    #[serial]
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
    #[serial]
    #[should_panic]
    fn panic_with_non_exist_file() {
        unsafe {
            std::env::set_var("CONFIG", "bar.toml");
        }
        let p = parse::<Conf>(None);
        unsafe {
            std::env::remove_var("CONFIG");
        }
    }
}
