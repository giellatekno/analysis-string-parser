// re-export items from tags
pub use analysis_tags::pos::Pos;
pub use analysis_tags::tag::{Tag, OwnedTag};

/// An Analysis String, `"+"` or `" "`-separated lemmas and tags. Used as input
/// to the generator, and output from the analyser.
///
/// E.g. "viessu+N+Sg+Nom", "fertet+v1+V+IV+Ind+Prs+Sg2" or , or, without the lemma:
/// "N+Sg+Nom"
#[derive(Debug)]
pub struct AnalysisParts {
    /// If the analysis contains a Pos.
    pub pos: Option<Pos>,

    /// All the individual parts. Contains the Pos as well.
    pub parts: Vec<AnalysisPart>,
}

impl PartialEq for AnalysisParts {
    fn eq(&self, other: &Self) -> bool {
        std::iter::zip(self.parts.iter(), other.parts.iter())
            .all(|(a, b)| a == b)
    }
}

impl Eq for AnalysisParts {}

impl std::hash::Hash for AnalysisParts {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.parts.hash(state);
    }
}

impl std::fmt::Display for AnalysisParts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut it = self.parts.iter();
        let first = it.next().expect("anaysis parts is never empty");
        write!(f, "{first}")?;
        for item in it {
            write!(f, "+{item}")?;
        }
        Ok(())
    }
}

impl serde::Serialize for AnalysisParts {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        use serde::ser::{SerializeStruct, SerializeSeq};
        let mut s = serializer.serialize_struct("parts", 2)?;
        s.serialize_field("pos", &self.pos)?;

        struct Parts<'a>(&'a Vec<AnalysisPart>);
        impl<'a> serde::Serialize for Parts<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer {
                let mut s = serializer.serialize_seq(Some(self.0.len()))?;
                for item in self.0 {
                    s.serialize_element(item)?;
                }
                s.end()
            }
        }
        let parts = Parts(&self.parts);
        s.serialize_field("parts", &parts)?;
        s.end()
    }
}

/// Parse an analysis parts string, with tags separated by `delim`.
pub fn parse_analysis_parts(
    s: &str,
    delim: &str,
) -> Option<AnalysisParts> {
    if s.is_empty() {
        return None;
    }

    let mut parts = vec![];
    let mut pos = None;

    for range in memmem_split(delim, s) {
        match Tag::from(&s[range.clone()]) {
            Tag::Unknown(inner) => {
                parts.push(AnalysisPart::Lemma(inner.to_string()));
            }
            tag => {
                parts.push(AnalysisPart::Tag(tag.to_owned()));
                if let Some(found_pos) = tag.pos() {
                    pos = Some(found_pos);
                }
            }
        }
    }

    Some(AnalysisParts { pos, parts })
}
/// A single part in an [`AnalysisParts`], it is either a lemma, or a tag.
#[derive(Debug, Eq, PartialEq)]
pub enum AnalysisPart {
    /// Unknown tag, therefore a Lemma.
    Lemma(String),
    /// A known tag.
    Tag(OwnedTag),
}

impl AnalysisParts {
    /// If there is a lemma in this analysisString, return it. The lemma can
    /// be split up, if it is a compound lemma, and is therefore returned
    /// as a newly allocated string, instead of a slice into the stored
    /// `AnalysisString::string`.
    pub fn lemma(&self) -> Option<String> {
        let s: String = self.parts
            .iter()
            .filter_map(|part| part.lemma())
            .map(|s| s.as_str())
            .collect();
        if !s.is_empty() { Some(s) } else { None }
    }

    //pub fn to_json(&self) -> serde_json::Value {
    //    serde_json::json!({
    //        "parts": self.parts,
    //    })
    //}
}

impl std::str::FromStr for AnalysisParts {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_analysis_parts(s, "+").ok_or(())
    }
}

impl AnalysisPart {
    /// Return the lemma range, if this is a lemma, otherwise None.
    fn lemma(&self) -> Option<&String> {
        match self {
            Self::Lemma(s) => Some(s),
            _ => None,
        }
    }
}

impl std::fmt::Display for AnalysisPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lemma(string) => write!(f, "{string}"),
            Self::Tag(owned_tag) => write!(f, "{owned_tag}"),
        }
    }
}

impl std::hash::Hash for AnalysisPart {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Lemma(string) => {
                state.write(string.as_bytes());
            }
            Self::Tag(owned_tag) => {
                owned_tag.hash(state);
            }
        }
    }
}

impl serde::Serialize for AnalysisPart {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("part", 1)?;
        match self {
            Self::Lemma(lemma) => s.serialize_field("lemma", lemma)?,
            Self::Tag(owned_tag) => s.serialize_field("tag", &owned_tag)?,
        }
        s.end()
    }
}

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
    use super::*;

    #[test]
    fn empty() {
        assert!(parse_analysis_parts("", "+").is_none());
    }

    #[test]
    fn only_lemma() {
        let parsed = parse_analysis_parts("lemma", "+").unwrap();
        assert!(parsed.pos.is_none());
        assert_eq!(
            parsed.parts.as_slice(),
            &[AnalysisPart::Lemma(String::from("lemma"))],
        );
    }

    #[test]
    fn only_tags() {
        let parsed = parse_analysis_parts("N+Neu+Pl+Indef", "+").unwrap();
        assert!(matches!(parsed.pos, Some(Pos::N)));
        assert_eq!(
            parsed.parts.as_slice(),
            &[
                AnalysisPart::Tag(OwnedTag::Pos(Pos::N)),
                AnalysisPart::Tag(OwnedTag::Neu),
                AnalysisPart::Tag(OwnedTag::Pl),
                AnalysisPart::Tag(OwnedTag::Indef),
            ],
        );
    }

    // {
    //     "lemma": "viessu",
    //     "pos": "N",
    //     "subclass": "",
    //     "tags": "Sg+Loc",
    //     "wordform": "viesus"
    // }
    // 
    // viessobargi: viessu+N+Cmp/SgNom+Cmp#bargi+N+NomAg+Sg+Gen 
    // 
    // {
    //     "lemma": "viessobargi",
    //     "pos": "N",
    //     "subclass": "NomAg",
    //     "tags": "Sg+Loc",
    //     "wordform": "viessobargis"
    // }
    // 
    // muitaluvvot: muitalit+V+TV+Der/PassL+V+IV+Inf 
    // 
    // {
    //     "lemma": "muitaluvvot",
    //     "pos": "V",
    //     "subclass": "",
    //     "tags": "Ind+Prs+Sg1",
    //     "wordform": "muitaluvvon"
    // }
}
