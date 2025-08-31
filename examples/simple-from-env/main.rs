#[derive(serde::Deserialize, Debug, Clone)]
pub struct Config  {
    port: u16,
    host: String,
}

fn main() {
    // simulate env set
    unsafe {
        std::env::set_var("port", "8080");
        std::env::set_var("host", "foo.bar");
    }

    let c = env_loader::parse::<Config>(None);
    println!("port: {:?}, host: {:?}", c.port, c.host);
    unsafe {
        std::env::remove_var("port");
        std::env::remove_var("host");
    }
}