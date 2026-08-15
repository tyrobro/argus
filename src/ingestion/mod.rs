use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::borrow::Cow;
use std::str;

#[derive(Debug, Default, PartialEq)]
pub struct Pacs008<'a> {
    pub msg_id: &'a str,
    pub instg_agt: &'a str,
    pub instd_agt: &'a str,
    pub tx_amount: f64,
}

enum Tag {
    MsgId,
    FinInstnId,
    InstdAmt,
    Other,
}

impl<'a> Pacs008<'a> {
    pub fn parse(xml: &'a str) -> Result<Self, &'static str> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut parsed = Pacs008::default();
        let mut current_tag = Tag::Other;
        let mut agent_count = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    current_tag = match e.name().as_ref() {
                        b"MsgId" => Tag::MsgId,
                        b"FinInstnId" => Tag::FinInstnId,
                        b"InstdAmt" => Tag::InstdAmt,
                        _ => Tag::Other,
                    };
                }
                Ok(Event::Text(e)) => {
                    if let Cow::Borrowed(bytes) = e.into_inner() {
                        if let Ok(text) = str::from_utf8(bytes) {
                            match current_tag {
                                Tag::MsgId => parsed.msg_id = text,
                                Tag::FinInstnId => {
                                    if agent_count == 0 {
                                        parsed.instg_agt = text;
                                        agent_count += 1;
                                    } else {
                                        parsed.instd_agt = text;
                                    }
                                }
                                Tag::InstdAmt => {
                                    parsed.tx_amount = text.parse::<f64>().unwrap_or(0.0);
                                }
                                Tag::Other => {}
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => return Err("Fatal XML parsing error"),
                _ => (),
            }
        }

        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pacs008_parsing() {
        let raw_xml = r#"
            <Document>
                <FIToFICstmrCdtTrf>
                    <GrpHdr>
                        <MsgId>TRX-987654321</MsgId>
                    </GrpHdr>
                    <CdtTrfTxInf>
                        <InstgAgt>
                            <FinInstnId>BANKUS33XXX</FinInstnId>
                        </InstgAgt>
                        <InstdAgt>
                            <FinInstnId>BANKGB22XXX</FinInstnId>
                        </InstdAgt>
                        <InstdAmt Ccy="USD">150000.50</InstdAmt>
                    </CdtTrfTxInf>
                </FIToFICstmrCdtTrf>
            </Document>
        "#;

        let result = Pacs008::parse(raw_xml).unwrap();

        assert_eq!(result.msg_id, "TRX-987654321");
        assert_eq!(result.instg_agt, "BANKUS33XXX");
        assert_eq!(result.instd_agt, "BANKGB22XXX");
        assert_eq!(result.tx_amount, 150000.50);
    }

    #[test]
    fn test_missing_tags_xml() {
        let raw_xml = "<Document><UnknownTag></UnknownTag></Document>";
        let result = Pacs008::parse(raw_xml).unwrap();
        
        assert_eq!(result.msg_id, "");
        assert_eq!(result.tx_amount, 0.0);
    }

    #[test]
    fn test_malformed_xml() {
        let raw_xml = "<Document><InvalidTag></Document>";
        let result = Pacs008::parse(raw_xml);
        
        assert!(result.is_err());
    }
}