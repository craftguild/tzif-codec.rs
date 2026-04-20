use std::{error::Error, fmt::Debug};

pub type TestResult = Result<(), Box<dyn Error>>;

pub trait AssertOk {
    type Output;

    fn assert_ok(self) -> Result<Self::Output, Box<dyn Error>>;
}

impl<T, E> AssertOk for Result<T, E>
where
    E: Error + 'static,
{
    type Output = T;

    fn assert_ok(self) -> Result<Self::Output, Box<dyn Error>> {
        self.map_err(Box::<dyn Error>::from)
    }
}

impl<T> AssertOk for Option<T> {
    type Output = T;

    fn assert_ok(self) -> Result<Self::Output, Box<dyn Error>> {
        self.ok_or_else(|| "expected Some(..), got None".into())
    }
}

#[allow(dead_code, reason = "not every integration test module expects errors")]
pub trait AssertErr {
    type Error;

    fn assert_err(self) -> Result<Self::Error, Box<dyn Error>>;
}

impl<T, E> AssertErr for Result<T, E>
where
    T: Debug,
{
    type Error = E;

    fn assert_err(self) -> Result<Self::Error, Box<dyn Error>> {
        match self {
            Ok(value) => Err(format!("expected Err(..), got Ok({value:?})").into()),
            Err(err) => Ok(err),
        }
    }
}
