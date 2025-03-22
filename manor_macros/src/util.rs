macro_rules! catch {
    ($result:expr) => {
        match $result {
            Ok(r) => r,
            Err(e) => {
                return proc_macro::TokenStream::from(darling::Error::from(e).write_errors());
            }
        }
    };
}

pub(crate) use catch;
