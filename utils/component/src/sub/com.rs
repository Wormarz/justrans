use super::super::Interface;
use anyhow::{Ok, Result};
use log::info;

pub struct Component;

impl Interface for Component {
    fn interface(&self) -> Result<()> {
        info!("Component");
        Ok(())
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_interface() {
        let component = Component;
        assert!(component.interface().is_ok());
    }
}