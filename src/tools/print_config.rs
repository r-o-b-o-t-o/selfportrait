use crate::{
    Result,
    config::Config,
};

pub fn run(config: &Config) -> Result<()> {
    println!("{}", config.to_json(false)?);
    Ok(())
}
