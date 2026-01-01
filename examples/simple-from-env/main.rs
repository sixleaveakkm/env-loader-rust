#[derive(serde::Deserialize, Debug, Clone)]
pub struct Config  {
    port: u16,
    host: String,
    embed: Embedded
}


#[derive(serde::Deserialize, Debug, Clone)]
pub struct Embedded {
    rds: String,
}

fn main() {
    // simulate env set
    unsafe {
        std::env::set_var("PORT", "8080");
        std::env::set_var("HOST", "foo.bar");
        std::env::set_var("EMBED_RDS", "rds.foo.bar");
    }

    let c = env_loader::parse::<Config>(None);
    println!("port: {:?}, host: {:?}, rds: {:?}", c.port, c.host, c.embed.rds);
    unsafe {
        std::env::remove_var("port");
        std::env::remove_var("host");
    }
}