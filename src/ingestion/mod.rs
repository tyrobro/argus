use crossbeam::queue::ArrayQueue;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::borrow::Cow;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::str;
use std::sync::Arc;

#[derive(Debug, Default, PartialEq)]
pub struct Pacs008<'a> {
    pub msg_id: &'a str,
    pub instg_agt: &'a str,
    pub instd_agt: &'a str,
    pub tx_amount: f64,
}

#[derive(Debug, Default, PartialEq, Copy, Clone)]
pub struct GraphEvent {
    pub msg_id: u64,
    pub src_node: u64,
    pub dst_node: u64,
    pub amount: f64,
}

enum Tag {
    MsgId,
    FinInstnId,
    InstdAmt,
    Other,
}

#[inline(always)]
fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
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

    pub fn into_graph_event(&self) -> GraphEvent {
        GraphEvent {
            msg_id: hash_str(self.msg_id),
            src_node: hash_str(self.instg_agt),
            dst_node: hash_str(self.instd_agt),
            amount: self.tx_amount,
        }
    }
}

pub struct IngestionPipeline {
    pub queue: Arc<ArrayQueue<GraphEvent>>,
}

impl IngestionPipeline {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(capacity)),
        }
    }

    pub fn push_event(&self, event: GraphEvent) -> Result<(), GraphEvent> {
        self.queue.push(event)
    }

    pub fn pop_event(&self) -> Option<GraphEvent> {
        self.queue.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

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

    #[test]
    fn test_into_graph_event() {
        let raw_xml = "<Document><FIToFICstmrCdtTrf><GrpHdr><MsgId>T1</MsgId></GrpHdr><CdtTrfTxInf><InstgAgt><FinInstnId>A</FinInstnId></InstgAgt><InstdAgt><FinInstnId>B</FinInstnId></InstdAgt><InstdAmt Ccy=\"USD\">10.0</InstdAmt></CdtTrfTxInf></FIToFICstmrCdtTrf></Document>";
        let parsed = Pacs008::parse(raw_xml).unwrap();
        let event = parsed.into_graph_event();

        assert_eq!(event.msg_id, hash_str("T1"));
        assert_eq!(event.src_node, hash_str("A"));
        assert_eq!(event.dst_node, hash_str("B"));
        assert_eq!(event.amount, 10.0);
    }

    #[test]
    fn test_concurrent_ingestion_pipeline() {
        let pipeline = IngestionPipeline::new(100_000);
        let queue_producer = pipeline.queue.clone();
        
        let raw_xml = "<Document><FIToFICstmrCdtTrf><GrpHdr><MsgId>T1</MsgId></GrpHdr><CdtTrfTxInf><InstgAgt><FinInstnId>A</FinInstnId></InstgAgt><InstdAgt><FinInstnId>B</FinInstnId></InstdAgt><InstdAmt Ccy=\"USD\">10.0</InstdAmt></CdtTrfTxInf></FIToFICstmrCdtTrf></Document>";
        let iterations = 10_000;

        let producer = thread::spawn(move || {
            for _ in 0..iterations {
                let parsed = Pacs008::parse(raw_xml).unwrap();
                let event = parsed.into_graph_event();
                while queue_producer.push(event).is_err() {}
            }
        });

        let mut processed_events = 0;
        let consumer = thread::spawn(move || {
            while processed_events < iterations {
                if pipeline.pop_event().is_some() {
                    processed_events += 1;
                }
            }
            processed_events
        });

        producer.join().unwrap();
        let total_processed = consumer.join().unwrap();
        
        assert_eq!(total_processed, iterations);
    }
}