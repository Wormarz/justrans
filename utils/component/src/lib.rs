pub mod sub;

pub trait Interface {
    fn interface(&self) -> anyhow::Result<()>;
}
