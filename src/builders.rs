use crate::counter::*;
use crate::*;

///
/// Builder for defining call patterns that will be recognized on a mock.
///
pub struct Each<M: Mock> {
    patterns: Vec<CallPattern<M>>,
}

impl<M: Mock + 'static> Each<M> {
    pub(crate) fn new() -> Self {
        Self { patterns: vec![] }
    }

    /// Set up a call pattern.
    /// The `matching` function receives a tuple representing the call arguments
    /// at hand. Its return value determines whether the defined call pattern matches the given arguments.
    ///
    /// Designed to work well with the [matching] macro.
    ///
    /// # Example
    ///
    /// ```rust
    /// #![feature(generic_associated_types)]
    /// use unimock::*;
    /// struct Foo;
    /// impl Mock for Foo {
    ///     /* ... */
    ///     # type Inputs<'i> = (String);
    ///     # type InputRefs<'i> = (&'i str);
    ///     # type Output = ();
    ///     # const NAME: &'static str = "Foo";
    ///     # fn input_refs<'i, 'o>((a0): &'o Self::Inputs<'i>) -> Self::InputRefs<'o> {
    ///     #     (a0.as_ref())
    ///     # }
    /// }
    ///
    /// fn test() {
    ///     let mock = Foo.mock(|each| { each.call(matching!("value")).returns_default(); });
    /// }
    /// ```
    pub fn call<'b, F>(&'b mut self, matching: F) -> Call<'b, M>
    where
        F: (for<'i> Fn(&M::InputRefs<'i>) -> bool) + Send + Sync + 'static,
    {
        let pat_index = self.patterns.len();
        self.patterns.push(CallPattern {
            pat_index,
            arg_matcher: Some(Box::new(matching)),
            call_counter: counter::CallCounter::new(counter::CountExpectation::None),
            output_factory: None,
        });

        Call {
            pattern: self.patterns.last_mut().unwrap(),
        }
    }

    pub(crate) fn build(self) -> Vec<CallPattern<M>> {
        self.patterns
    }
}

///
/// Builder for configuring a specific call pattern.
///
pub struct Call<'b, M: Mock> {
    pattern: &'b mut CallPattern<M>,
}

impl<'b, M> Call<'b, M>
where
    M: Mock + 'static,
{
    /// Specify the output of the call pattern by providing a value.
    /// The output type must implement `Clone` and cannot contain non-static references.
    pub fn returns(self, value: M::Output) -> Self
    where
        M::Output: Send + Sync + Clone + 'static,
    {
        self.pattern.output_factory = Some(Box::new(move |_| value.clone()));
        self
    }

    /// Specify the output of the call pattern by calling `Default::default()`.
    pub fn returns_default(self) -> Self
    where
        M::Output: Default,
    {
        self.pattern.output_factory = Some(Box::new(|_| Default::default()));
        self
    }

    /// Specify the output of the call pattern by invoking the given closure that
    /// can then compute it based on input parameters.
    pub fn answers<F>(self, f: F) -> Self
    where
        F: (for<'i> Fn(M::Inputs<'i>) -> M::Output) + Send + Sync + 'static,
    {
        self.pattern.output_factory = Some(Box::new(f));
        self
    }

    /// Prevent this call pattern from succeeding by explicitly panicking with a custom message.
    pub fn panics(self, message: impl Into<String>) -> Self {
        let message = message.into();
        self.pattern.output_factory = Some(Box::new(move |_| panic!("{}", message)));
        self
    }

    /// Expect this call pattern to never be called.
    pub fn never(self) -> Self {
        self.pattern
            .call_counter
            .set_expectation(CountExpectation::Exactly(0));
        self
    }

    /// Expect this call pattern to be called exactly once.
    pub fn once(self) -> Self {
        self.pattern
            .call_counter
            .set_expectation(CountExpectation::Exactly(1));
        self
    }

    /// Expect this call pattern to be called exactly the specified number of times.
    pub fn times(self, times: usize) -> Self {
        self.pattern
            .call_counter
            .set_expectation(CountExpectation::Exactly(times));
        self
    }

    /// Expect this call pattern to be called at least the specified number of times.
    pub fn at_least(self, times: usize) -> Self {
        self.pattern
            .call_counter
            .set_expectation(CountExpectation::AtLeast(times));
        self
    }
}
