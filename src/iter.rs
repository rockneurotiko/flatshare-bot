
pub trait LangIter : Iterator {
    fn listify(&mut self, sep: &str, last_sep: &str) -> String
        where Self::Item: ::std::fmt::Display
    {
        use std::fmt::Write;

        match self.next() {
            None => String::new(),
            Some(first) => {
                // Allocate string buffer with lower capacity bound to reduce
                // number of allocations.
                let (lo, _) = self.size_hint();
                let min_cap = sep.len() * lo + last_sep.len();

                // Fill output string buffer
                let mut out = String::with_capacity(min_cap);
                write!(&mut out, "{}", first).unwrap();

                let mut last = self.next();

                while let Some(prev) = last {
                    let next = self.next();
                    match next {
                        None => {
                            write!(&mut out, "{}{}", last_sep, prev).unwrap();
                        },
                        Some(_) => {
                            write!(&mut out, "{}{}", sep, prev).unwrap();
                        }
                    }

                    last = next;
                }

                out
            }
        }
    }

}

impl<T: ?Sized> LangIter for T where T: Iterator { }
