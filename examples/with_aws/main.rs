#[derive(serde::Deserialize, Debug, Clone)]
pub struct Config  {
    port: u16,
    host: String,
    user: User,
    password: String
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct User {
    pub username: String,
    pub password: String,
}

fn main() {
    // simulate env set
    unsafe {
        std::env::set_var("port", "8080");
    }

    let c = env_loader::parse::<Config>(None);
    println!("port: {:?}, host: {:?}", c.port, c.host);
    unsafe {
        std::env::remove_var("port");
    }
}