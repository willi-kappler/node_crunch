
#[derive(Debug, Fail)]
enum ServerError {
    #[fail(display = "Could not bind address: {}", address)]
    BindError {
        address: String
    },
}
