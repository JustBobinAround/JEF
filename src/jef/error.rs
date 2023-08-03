use std::error::Error;
#[derive(Debug)]
pub struct JefError {
    message: String,
}
impl Error for JefError {}

impl JefError {
    pub fn new(message: &str) -> Box<JefError>{
        let error = JefError{
            message: message.to_string(),
        };
        Box::new(error)
    }
}

impl std::fmt::Display for JefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

