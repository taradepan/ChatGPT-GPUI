#[derive(Clone, Debug)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub id: usize,
    pub role: Role,
    pub content: String,
}
