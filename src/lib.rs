mod memmem_split;

// re-export items from tags
pub use analysis_tags::pos::Pos;
pub use analysis_tags::tag::{OwnedTag, Tag};
pub use memmem_split::memmem_split;

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
        std::iter::zip(self.parts.iter(), other.parts.iter()).all(|(a, b)| a == b)
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
        S: serde::Serializer,
    {
        use serde::ser::{SerializeSeq, SerializeStruct};
        let mut s = serializer.serialize_struct("parts", 2)?;
        s.serialize_field("pos", &self.pos)?;

        struct Parts<'a>(&'a Vec<AnalysisPart>);
        impl<'a> serde::Serialize for Parts<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
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

/// Parse an analysis parts string, with tags separated by `"+"`.
pub fn parse_analysis_parts(s: &str) -> Option<AnalysisParts> {
    if s.is_empty() {
        return None;
    }

    let mut parts = vec![];
    let mut pos = None;

    for r1 in memmem_split("#", s) {
        for r2 in memmem_split("+", &s[r1.clone()]) {
            match Tag::from(&s[r1.clone()][r2]) {
                Tag::Unknown(inner) => {
                    parts.push(AnalysisPart::Lemma(inner.to_string()));
                }
                tag @ Tag::Pos(new_pos) => {
                    parts.push(AnalysisPart::Tag(tag.to_owned()));
                    pos = Some(new_pos);
                }
                tag => parts.push(AnalysisPart::Tag(tag.to_owned())),
            }
        }

        parts.push(AnalysisPart::WordBoundry);
    }

    // pop the last wordboundry
    parts.pop();

    Some(AnalysisParts { pos, parts })
}

/// A single part in an [`AnalysisParts`], it is either a lemma, or a tag.
#[derive(Debug, Eq, PartialEq)]
pub enum AnalysisPart {
    /// Unknown tag, therefore a Lemma.
    Lemma(String),
    /// A known tag.
    Tag(OwnedTag),
    /// A Word boundry, i.e. the "#" character
    WordBoundry,
}

impl AnalysisParts {
    /// If there is a lemma in this analysisString, return it. The lemma can
    /// be split up, if it is a compound lemma, and is therefore returned
    /// as a newly allocated string, instead of a slice into the stored
    /// `AnalysisString::string`.
    pub fn lemma(&self) -> Option<String> {
        let s: String = self
            .parts
            .iter()
            .filter_map(|part| part.lemma())
            .map(|s| s.as_str())
            .collect();
        if !s.is_empty() {
            Some(s)
        } else {
            None
        }
    }

    pub fn is_compound(&self) -> bool {
        self.parts.contains(&AnalysisPart::WordBoundry)
    }

    pub fn last_word_boundrary_pos(&self) -> Option<usize> {
        let mut pos = None;
        for (i, part) in self.parts.iter().enumerate() {
            if matches!(part, AnalysisPart::WordBoundry) {
                pos = Some(i);
            }
        }
        pos
    }

    /// The stringified analysis part that is only missing the last part, to be sent
    /// for generation.
    /// When it's not a compound, we can just pass the lemma itself, but when it is a
    /// compound, we have to send all the lemmas with all the tags, up to, but not
    /// including the last one.
    pub fn generation_string_prefix(&self) -> String {
        if let Some(i) = self.last_word_boundrary_pos() {
            use std::fmt::Write;
            // i is the index of the last WordBoundry, we want everything up to that,
            // and including the next part, which SHOULD BE the lemma of the last
            // compound word
            let mut out = String::new();

            for part in self.parts.iter().take(i + 2) {
                let _ = match part {
                    AnalysisPart::WordBoundry => write!(out, "#"),
                    AnalysisPart::Tag(OwnedTag::Cmp) => write!(out, "Cmp"),
                    part => write!(out, "{part}+"),
                };
            }

            // remove last "+", if it ends with a plus
            if out.ends_with('+') {
                out.pop();
            }
            out
        } else {
            if let Some(lemma) = self.lemma() {
                lemma
            } else {
                panic!("what to generate here?");
            }
        }
    }
}

impl std::str::FromStr for AnalysisParts {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_analysis_parts(s).ok_or(())
    }
}

impl AnalysisPart {
    /// Return the lemma, if this is a lemma, otherwise None.
    fn lemma(&self) -> Option<&String> {
        match self {
            Self::Lemma(s) => Some(s),
            _ => None,
        }
    }

    /// Return a reference to the `OwnedTag`, if this part is a `Tag`, otherwise `None`.
    pub fn tag(&self) -> Option<&OwnedTag> {
        match self {
            Self::Tag(owned_tag) => Some(owned_tag),
            _ => None,
        }
    }
}

impl std::fmt::Display for AnalysisPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lemma(string) => write!(f, "{string}"),
            Self::Tag(owned_tag) => write!(f, "{owned_tag}"),
            Self::WordBoundry => write!(f, "#"),
        }
    }
}

impl std::hash::Hash for AnalysisPart {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Lemma(string) => state.write(string.as_bytes()),
            Self::Tag(owned_tag) => owned_tag.hash(state),
            Self::WordBoundry => state.write_u8(b'#'),
        }
    }
}

impl serde::Serialize for AnalysisPart {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("part", 1)?;
        match self {
            Self::Lemma(lemma) => s.serialize_field("lemma", lemma)?,
            Self::Tag(owned_tag) => s.serialize_field("tag", &owned_tag)?,
            Self::WordBoundry => s.serialize_field("wordboundry", "#")?,
        }
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        assert!(parse_analysis_parts("").is_none());
    }

    #[test]
    fn only_lemma() {
        let parsed = parse_analysis_parts("lemma").unwrap();
        assert!(parsed.pos.is_none());
        assert_eq!(
            parsed.parts.as_slice(),
            &[AnalysisPart::Lemma(String::from("lemma"))],
        );
    }

    #[test]
    fn only_tags() {
        let parsed = parse_analysis_parts("N+Neu+Pl+Indef").unwrap();
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

    #[test]
    fn compound() {
        let parsed = parse_analysis_parts("skuvla+N+Cmp/SgNom+Cmp#gohppa+N+Sg+Nom").unwrap();
        assert_eq!(
            parsed.parts.as_slice(),
            &[
                AnalysisPart::Lemma(String::from("skuvla")),
                AnalysisPart::Tag(OwnedTag::Pos(Pos::N)),
                AnalysisPart::Tag(OwnedTag::CmpX(String::from("SgNom"))),
                AnalysisPart::Tag(OwnedTag::Cmp),
                AnalysisPart::WordBoundry,
                AnalysisPart::Lemma(String::from("gohppa")),
                AnalysisPart::Tag(OwnedTag::Pos(Pos::N)),
                AnalysisPart::Tag(OwnedTag::Sg),
                AnalysisPart::Tag(OwnedTag::Nom),
            ],
        )
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
