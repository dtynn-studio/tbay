pub trait Monitor {
    fn key(&self) -> &str;
    fn deps(&self) -> Vec<&str>;
    fn calc(&self);
    fn update(&mut self) -> bool;
}
