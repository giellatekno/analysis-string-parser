/// Split the string [`s`] by [`delim`], and return an iterator that yields
/// [`std::ops::Range<usize>`] that slices the individual pieces of [`s`].
///
/// For exampe:
/// ```rust
/// use analysis_string_parser::memmem_split;
/// let s = "v1+V+IV+Ind+Prs+Sg2";
/// let mut it = memmem_split("+", s);
/// assert_eq!(&s[it.next().unwrap()], "v1");
/// assert_eq!(&s[it.next().unwrap()], "V");
/// assert_eq!(&s[it.next().unwrap()], "IV");
/// assert_eq!(&s[it.next().unwrap()], "Ind");
/// assert_eq!(&s[it.next().unwrap()], "Prs");
/// assert_eq!(&s[it.next().unwrap()], "Sg2");
/// assert_eq!(it.next(), None);
/// ```
pub fn memmem_split<'a>(
    delim: &'a str,
    s: &'a str,
) -> impl Iterator<Item = std::ops::Range<usize>> {
    let finder = memchr::memmem::Finder::new(delim).into_owned();
    let mut it = finder.find_iter(s.as_bytes()).into_owned();
    let mut prev = 0;
    let mut done = false;

    std::iter::from_fn(move || match it.next() {
        Some(i) => {
            let res = prev..i;
            prev = i + delim.len();
            Some(res)
        }
        None => {
            if done {
                None
            } else {
                done = true;
                Some(prev..s.len())
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::memmem_split;

    fn run_single(input: &str, delim: &str, expected: Vec<&str>) {
        let actual: Vec<_> = memmem_split(delim, input)
            .map(|range| &input[range])
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn single_yields_once_on_empty() {
        run_single("", "+", vec![""]);
    }

    #[test]
    fn single_yields_once_on_no_splits() {
        run_single("this is a string", "+", vec!["this is a string"]);
    }

    #[test]
    fn single_yields_twice_on_single_split() {
        run_single("this+is", "+", vec!["this", "is"]);
    }

    #[test]
    fn single_yields_empty_end() {
        run_single("this+is+", "+", vec!["this", "is", ""]);
    }

    #[test]
    fn single_yields_empty_middle() {
        run_single("this++is", "+", vec!["this", "", "is"]);
    }

    #[test]
    fn single_yields_multiple_empty_middle() {
        run_single("this++++is", "+", vec!["this", "", "", "", "is"]);
    }

    #[test]
    fn multilevel_empty() {
        let input = "";
        let mut actual = vec![];
        for r1 in memmem_split("#", input) {
            for r2 in memmem_split("+", &input[r1.clone()]) {
                actual.push(&input[r1.clone()][r2]);
            }
        }

        let expected: Vec<&str> = vec![""];
        assert_eq!(actual, expected);
    }
}
