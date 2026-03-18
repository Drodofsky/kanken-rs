use nom::bytes::complete::take;
use nom::error::{Error, ErrorKind};
use nom::multi::many0;
use nom::number::complete::le_i32;
use nom::sequence::preceded;
use nom::{IResult, Parser};

use nom::combinator::map;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct QuestionFile {
    pub record_list_size: i32,
    pub items: Vec<QuestionItem>,
    pub field_id: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionField {
    pub items: Vec<QuestionItem>,
    pub field_info: FieldInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AnswerType {
    Handwriting = 1, // 手書き
    Number = 2,      // 数字 (画数・筆順)
    Choice = 3,      // 選択
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AllocationScore {
    OnePoint = 1,
    TwoPoints = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DispDictionaryType {
    None = 0,
    Kanji = 1,
    Yojijukugo = 2,
    YojijukugoMeaning = 3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldInfo {
    pub field_id: i32,
    pub level: Kyu,
    pub name: String, // keep as String — 47 unique values
    pub preamble: String,
    pub answer_type: AnswerType,
    pub answer_count: i32, // 1, 2, or 7
    pub correct_type: i32,
    pub disp_correct_answer_type: i32,
    pub disp_dictionary_type: DispDictionaryType,
    pub count_per_exam: i32,
    pub allocation_score: AllocationScore,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionItem {
    pub field_id: i32,
    pub question_id: i32,
    pub level: Kyu,
    pub year: i32,
    pub kind: i32,
    pub matter: String,
    pub format: String,
    pub large_part: i32,
    pub middle_part: i32,
    pub small_part: i32,
    pub sentence: String,
    pub answer_choices: String,
    pub correct_answer_list: Vec<String>,
    pub use_word_list: Vec<String>,
    pub selected_index: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Kyu {
    Kyu10 = 12,
    Kyu9 = 11,
    Kyu8 = 10,
    Kyu7 = 9,
    Kyu6 = 8,
    Kyu5 = 7,
    Kyu4 = 6,
    Kyu3 = 5,
    Jun2 = 4,
    Kyu2 = 3,
    Jun1 = 2,
    Kyu1 = 1,
}
impl TryFrom<i32> for Kyu {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            12 => Ok(Kyu::Kyu10),
            11 => Ok(Kyu::Kyu9),
            10 => Ok(Kyu::Kyu8),
            9 => Ok(Kyu::Kyu7),
            8 => Ok(Kyu::Kyu6),
            7 => Ok(Kyu::Kyu5),
            6 => Ok(Kyu::Kyu4),
            5 => Ok(Kyu::Kyu3),
            4 => Ok(Kyu::Jun2),
            3 => Ok(Kyu::Kyu2),
            2 => Ok(Kyu::Jun1),
            1 => Ok(Kyu::Kyu1),
            _ => Err(format!("Invalid Kyu value: {}", value)),
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    NomError(String),
    Remaining {
        field_id: i32,
        bytes: usize,
    },
    SizeMismatch {
        field_id: i32,
        declared: i32,
        actual: usize,
    },
    IdMismatch {
        expected: i32,
        got: i32,
    },
    FieldCountMismatch {
        csv_count: usize,
        dat_count: usize,
    },
    CsvError(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NomError(e) => write!(f, "parse error: {e}"),
            ParseError::Remaining { field_id, bytes } => {
                write!(f, "field {field_id}: {bytes} remaining bytes")
            }
            ParseError::SizeMismatch {
                field_id,
                declared,
                actual,
            } => write!(
                f,
                "field {field_id}: declared {declared} but got {actual} items"
            ),
            ParseError::IdMismatch { expected, got } => {
                write!(f, "field id mismatch: expected {expected} got {got}")
            }
            ParseError::FieldCountMismatch {
                csv_count,
                dat_count,
            } => write!(
                f,
                "csv has {csv_count} fields but got {dat_count} dat files"
            ),
            ParseError::CsvError(e) => write!(f, "csv error: {e}"),
        }
    }
}

pub fn parse_all_fields(
    dat_files: &[&[u8]],
    csv_str: &str,
) -> Result<Vec<QuestionField>, ParseError> {
    let field_infos = parse_field_master(csv_str);

    if field_infos.len() != dat_files.len() {
        return Err(ParseError::FieldCountMismatch {
            csv_count: field_infos.len(),
            dat_count: dat_files.len(),
        });
    }

    field_infos
        .into_iter()
        .zip(dat_files.iter())
        .map(|(info, &dat)| {
            let (rem, file) =
                parse_question_file(dat).map_err(|e| ParseError::NomError(format!("{e:?}")))?;

            if !rem.is_empty() {
                return Err(ParseError::Remaining {
                    field_id: info.field_id,
                    bytes: rem.len(),
                });
            }

            if file.items.len() as i32 != file.record_list_size {
                return Err(ParseError::SizeMismatch {
                    field_id: info.field_id,
                    declared: file.record_list_size,
                    actual: file.items.len(),
                });
            }

            if file.field_id != info.field_id {
                return Err(ParseError::IdMismatch {
                    expected: info.field_id,
                    got: file.field_id,
                });
            }

            Ok(QuestionField {
                field_info: info,
                items: file.items,
            })
        })
        .collect()
}
pub fn parse_field_master(csv_str: &str) -> Vec<FieldInfo> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(csv_str.as_bytes());

    rdr.records()
        .filter_map(|r| r.ok())
        .filter_map(|r| {
            let field_id: i32 = r[0].trim().parse().ok()?;
            let level = r[1]
                .trim()
                .parse::<i32>()
                .ok()
                .and_then(|v| Kyu::try_from(v).ok())?;
            let answer_type = match r[4].trim().parse::<i32>().ok()? {
                1 => AnswerType::Handwriting,
                2 => AnswerType::Number,
                3 => AnswerType::Choice,
                _ => return None,
            };
            let disp_dictionary_type = match r[8].trim().parse::<i32>().ok()? {
                0 => DispDictionaryType::None,
                1 => DispDictionaryType::Kanji,
                2 => DispDictionaryType::Yojijukugo,
                3 => DispDictionaryType::YojijukugoMeaning,
                _ => return None,
            };
            let allocation_score = match r[10].trim().parse::<i32>().ok()? {
                1 => AllocationScore::OnePoint,
                2 => AllocationScore::TwoPoints,
                _ => return None,
            };
            Some(FieldInfo {
                field_id,
                level,
                name: r[2].to_string(),
                preamble: r[3].to_string(),
                answer_type,
                answer_count: r[5].trim().parse().ok()?,
                correct_type: r[6].trim().parse().ok()?,
                disp_correct_answer_type: r[7].trim().parse().ok()?,
                disp_dictionary_type,
                count_per_exam: r[9].trim().parse().ok()?,
                allocation_score,
            })
        })
        .collect()
}
pub fn parse_question_item(input: &[u8]) -> IResult<&[u8], QuestionItem> {
    let (input, field_id) = le_i32.parse(input)?;
    let (input, question_id) = le_i32.parse(input)?;
    let (input, level) = parse_kyu.parse(input)?;
    let (input, year) = le_i32.parse(input)?;
    let (input, kind) = le_i32.parse(input)?;

    let (input, matter) = parse_string(input)?;
    let (input, format) = parse_string(input)?;

    let (input, large_part) = le_i32.parse(input)?;
    let (input, middle_part) = le_i32.parse(input)?;
    let (input, small_part) = le_i32.parse(input)?;

    let (input, sentence) = parse_string(input)?;
    let (input, answer_choices) = parse_string(input)?;

    let (input, correct_answer_list) = parse_string_array(input)?;
    let (input, use_word_list) = parse_string_array(input)?;

    let (input, selected_index) = le_i32.parse(input)?;

    Ok((
        input,
        QuestionItem {
            field_id,
            question_id,
            level,
            year,
            kind,
            matter,
            format,
            large_part,
            middle_part,
            small_part,
            sentence,
            answer_choices,
            correct_answer_list,
            use_word_list,
            selected_index,
        },
    ))
}

fn parse_kyu(input: &[u8]) -> IResult<&[u8], Kyu> {
    let (input, value) = le_i32.parse(input)?;

    let kyu = match value {
        12 => Kyu::Kyu10,
        11 => Kyu::Kyu9,
        10 => Kyu::Kyu8,
        9 => Kyu::Kyu7,
        8 => Kyu::Kyu6,
        7 => Kyu::Kyu5,
        6 => Kyu::Kyu4,
        5 => Kyu::Kyu3,
        4 => Kyu::Jun2,
        3 => Kyu::Kyu2,
        2 => Kyu::Jun1,
        1 => Kyu::Kyu1,
        _ => return Err(nom::Err::Error(Error::new(input, ErrorKind::MapRes))),
    };

    Ok((input, kyu))
}

impl Kyu {
    pub const ALL: &'static [Kyu] = &[
        Kyu::Kyu10,
        Kyu::Kyu9,
        Kyu::Kyu8,
        Kyu::Kyu7,
        Kyu::Kyu6,
        Kyu::Kyu5,
        Kyu::Kyu4,
        Kyu::Kyu3,
        Kyu::Jun2,
        Kyu::Kyu2,
        Kyu::Jun1,
        Kyu::Kyu1,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Kyu::Kyu10 => "10級",
            Kyu::Kyu9 => "9級",
            Kyu::Kyu8 => "8級",
            Kyu::Kyu7 => "7級",
            Kyu::Kyu6 => "6級",
            Kyu::Kyu5 => "5級",
            Kyu::Kyu4 => "4級",
            Kyu::Kyu3 => "3級",
            Kyu::Jun2 => "準2級",
            Kyu::Kyu2 => "2級",
            Kyu::Jun1 => "準1級",
            Kyu::Kyu1 => "1級",
        }
    }
}

fn parse_string_array(input: &[u8]) -> IResult<&[u8], Vec<String>> {
    let (mut input, size) = le_i32.parse(input)?;
    let mut out = Vec::with_capacity(size as usize);

    for _ in 0..size {
        let (rem, value) = parse_string(input)?;
        input = rem;
        out.push(value);
    }

    Ok((input, out))
}

pub fn parse_question_file(input: &[u8]) -> IResult<&[u8], QuestionFile> {
    let (rem, res) = preceded(
        parse_question_file_start,
        (parse_record_list_size, many0(parse_question_item), le_i32),
    )
    .parse(input)?;

    Ok((
        rem,
        QuestionFile {
            record_list_size: res.0,
            items: res.1,
            field_id: res.2,
        },
    ))
}

fn parse_question_file_start(input: &[u8]) -> IResult<&[u8], ()> {
    // parse name
    let (rem, name) = preceded(skip_header, parse_unity_string).parse(input)?;
    if !name.starts_with(b"KankenQuestionSO_") {
        // return Err(b"not a KankenQuestionSO file");

        return Err(nom::Err::Error(Error::new(name, ErrorKind::Tag)));
    }
    Ok((rem, ()))
}

fn parse_record_list_size(input: &[u8]) -> IResult<&[u8], i32> {
    le_i32(input)
}

/// Skips the Unity MonoBehaviour native header (56 bytes)
fn skip_header(input: &[u8]) -> IResult<&[u8], ()> {
    map(take(0x1Cusize), |_| ()).parse(input)
}

fn parse_unity_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, len) = le_i32.parse(input)?;
    if len < 0 {
        // handle null string
        return Ok((input, &[]));
    }
    let len = len as usize;

    let (input, bytes) = take(len).parse(input)?;

    // Unity aligns strings to 4 bytes
    let padding = (4 - (len % 4)) % 4;
    let (input, _) = take(padding).parse(input)?;

    Ok((input, bytes))
}
fn parse_string(input: &[u8]) -> IResult<&[u8], String> {
    let (input, bytes) = parse_unity_string(input)?;

    let s = std::str::from_utf8(bytes)
        .map_err(|_| nom::Err::Error(nom::error::Error::new(bytes, ErrorKind::MapRes)))?
        .to_string();

    Ok((input, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_first_file() {
        let data = include_bytes!("../../data/KankenQuestionSO_120-resources.assets-1003.dat");
        let (rem, question_file) = parse_question_file(data).unwrap();
        assert_eq!(question_file.record_list_size, 1300);
        assert_eq!(question_file.items.len(), 1300);
        assert_eq!(question_file.field_id, 120);
        assert!(rem.is_empty());
    }
    #[test]
    fn on_second_file() {
        let data = include_bytes!("../../data/KankenQuestionSO_119-resources.assets-1001.dat");
        let (rem, question_file) = parse_question_file(data).unwrap();
        assert_eq!(question_file.record_list_size, 650);
        assert_eq!(question_file.items.len(), 650);
        assert_eq!(question_file.field_id, 119);
        assert!(rem.is_empty());
    }

    #[test]
    fn test_skip_header() {
        let data = vec![0u8; 100];
        let (rest, ()) = skip_header(&data).unwrap();
        assert_eq!(rest.len(), 72);
    }
    fn unity_string_bytes(s: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(s.len() as i32).to_le_bytes());
        out.extend_from_slice(s);

        let padding = (4 - (s.len() % 4)) % 4;
        out.extend(std::iter::repeat(0).take(padding));
        out
    }

    #[test]
    fn test_parse_question_file_start_ok() {
        let mut data = vec![0u8; 0x1C];
        data.extend(unity_string_bytes(b"KankenQuestionSO_114"));

        let (rem, ()) = parse_question_file_start(&data).unwrap();
        assert!(rem.is_empty());
    }

    #[test]
    fn test_parse_question_file_start_wrong_name() {
        let mut data = vec![0u8; 0x1C];
        data.extend(unity_string_bytes(b"KankenDictionarySO"));

        assert!(parse_question_file_start(&data).is_err());
    }

    #[test]
    fn test_all_question_files() {
        for id in 1..=120 {
            let path = format!("../data/KankenQuestionSO_{}.dat", id);

            println!("testing {}", path);

            let data = std::fs::read(&path).unwrap_or_else(|_| panic!("missing file: {}", path));

            let (rem, file) = parse_question_file(&data)
                .unwrap_or_else(|e| panic!("parse failed for {}: {:?}", path, e));

            // ✔ size matches
            assert_eq!(
                file.items.len() as i32,
                file.record_list_size,
                "size mismatch in {}",
                path
            );

            // ✔ no remaining bytes
            assert!(
                rem.is_empty(),
                "remaining bytes in {}: {} bytes",
                path,
                rem.len()
            );

            // ✔ id matches filename
            assert_eq!(file.field_id, id, "field_id mismatch in {}", path);
        }
    }
    #[test]
    fn dump_all_question_files_to_json() {
        let mut all: Vec<QuestionItem> = Vec::new();

        for id in 1..=120 {
            let path = format!("../data/KankenQuestionSO_{}.dat", id);

            let data = std::fs::read(&path).unwrap_or_else(|_| panic!("missing file: {}", path));

            let (rem, mut file) = parse_question_file(&data)
                .unwrap_or_else(|e| panic!("parse failed for {}: {:?}", path, e));

            assert!(
                rem.is_empty(),
                "remaining bytes in {}: {} bytes",
                path,
                rem.len()
            );

            all.append(&mut file.items);
        }

        std::fs::write(
            "../data/combined_question_items.json",
            serde_json::to_string(&all).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn dump_combined_fields_to_json() {
        let csv = std::fs::read_to_string("../data/kanken_field_master.csv").unwrap();

        let dats: Vec<Vec<u8>> = (1..=120)
            .map(|id| std::fs::read(format!("../data/KankenQuestionSO_{id}.dat")).unwrap())
            .collect();
        let dat_refs: Vec<&[u8]> = dats.iter().map(|v| v.as_slice()).collect();

        let fields = parse_all_fields(&dat_refs, &csv).unwrap();

        assert_eq!(fields.len(), 120);

        std::fs::write(
            "../data/combined_fields.json",
            serde_json::to_string(&fields).unwrap(),
        )
        .unwrap();
    }
}
