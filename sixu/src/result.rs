use nom::error::VerboseError;
use nom::IResult;

pub type SixuResult<I, O> = IResult<I, O, VerboseError<I>>;
