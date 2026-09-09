//! Network access for SerialClon.rs. Off by default: every function here runs
//! only on an explicit user action (Web Access, BLAST, Import Codon Usage)
//! and fails with a plain message when offline.
//!
//! The 2013-era page scraping is replaced by the documented APIs
//! (`PLAN.md` §2): NCBI E-utilities `efetch` for records, the ENA browser API
//! for EMBL records, and the NCBI BLAST URL API (`CMD=Put` / `CMD=Get`) for
//! remote searches, polled at the rate NCBI asks for (wait the estimated
//! time, then not more than once a minute per request).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Errors from the network layer.
#[derive(Debug, thiserror::Error)]
pub enum NetError {
    /// Could not reach the server.
    #[error(
        "cannot reach {0}: {1}. Check the network connection (SerialClon only goes online when you ask it to)."
    )]
    Unreachable(String, String),
    /// The server answered with an error status.
    #[error("{0} answered {1}")]
    Status(String, u16),
    /// The answer was not what the API documents.
    #[error("{0}")]
    Malformed(String),
    /// Polled too soon.
    #[error("BLAST asks not to poll a request more than once a minute; try again in {0} s")]
    TooSoon(u64),
}

/// Result alias.
pub type Result<T> = std::result::Result<T, NetError>;

const NCBI_EFETCH: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi";
const ENA_BROWSER: &str = "https://www.ebi.ac.uk/ena/browser/api";
const BLAST: &str = "https://blast.ncbi.nlm.nih.gov/Blast.cgi";
/// Two-sequence searches ("Use BlastAlign.cgi to submit blast2seq searches").
const BLAST_ALIGN: &str = "https://blast.ncbi.nlm.nih.gov/BlastAlign.cgi";
/// NCBI: "do not poll for any single RID more often than once a minute".
const POLL_INTERVAL: Duration = Duration::from_secs(60);

/// The HTTP client, with NCBI's `tool` / `email` identification.
pub struct Client {
    http: reqwest::blocking::Client,
    tool: String,
    email: String,
    last_poll: Mutex<HashMap<String, Instant>>,
}

/// NCBI databases we fetch from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NcbiDb {
    /// Nucleotide (GenBank flat file).
    Nuccore,
    /// Protein (GenPept flat file).
    Protein,
}

impl NcbiDb {
    fn name(self) -> &'static str {
        match self {
            Self::Nuccore => "nuccore",
            Self::Protein => "protein",
        }
    }
    fn rettype(self) -> &'static str {
        match self {
            Self::Nuccore => "gb",
            Self::Protein => "gp",
        }
    }
}

/// A submitted BLAST request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlastJob {
    /// Request id.
    pub rid: String,
    /// Estimated time to completion, seconds.
    pub rtoe: u64,
}

/// The state of a BLAST request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlastStatus {
    /// Still running.
    Waiting,
    /// Finished with hits.
    Ready,
    /// Finished, no hits.
    NoHits,
    /// Failed on the server.
    Failed,
    /// Expired or unknown RID.
    Unknown,
}

/// What to submit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlastRequest {
    /// `blastn`, `blastp`, `blastx`, `tblastn`, `megablast`.
    pub program: String,
    /// `nt`, `nr`, … — `None` for a two-sequence alignment (`subject` set).
    pub database: Option<String>,
    /// The query, FASTA or bare sequence.
    pub query: String,
    /// The subject for BLAST2Seq.
    pub subject: Option<String>,
    /// Maximum hits.
    pub hitlist_size: u32,
}

impl Client {
    /// A client identifying itself as `tool` with a contact `email` (NCBI
    /// asks for both) and a 60 s timeout.
    pub fn new(tool: &str, email: &str) -> Result<Client> {
        Self::with_proxy(tool, email, None)
    }

    /// Like [`Client::new`], routed through `proxy` (an `http://host:port`
    /// URL) instead of the system proxy settings.
    pub fn with_proxy(tool: &str, email: &str, proxy: Option<&str>) -> Result<Client> {
        let mut builder = reqwest::blocking::Client::builder()
            .user_agent(format!(
                "{tool} (+https://github.com/luksgrin/SerialClon.rs)"
            ))
            .timeout(Duration::from_secs(60));
        if let Some(url) = proxy.map(str::trim).filter(|u| !u.is_empty()) {
            let proxy = reqwest::Proxy::all(url)
                .map_err(|e| NetError::Unreachable("proxy".into(), e.to_string()))?;
            builder = builder.proxy(proxy);
        }
        let http = builder
            .build()
            .map_err(|e| NetError::Unreachable("client".into(), e.to_string()))?;
        Ok(Client {
            http,
            tool: tool.to_owned(),
            email: email.to_owned(),
            last_poll: Mutex::new(HashMap::new()),
        })
    }

    fn get(&self, host: &str, url: &str, query: &[(&str, String)]) -> Result<String> {
        let resp = self
            .http
            .get(url)
            .query(query)
            .send()
            .map_err(|e| NetError::Unreachable(host.to_owned(), e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(NetError::Status(host.to_owned(), status.as_u16()));
        }
        resp.text()
            .map_err(|e| NetError::Unreachable(host.to_owned(), e.to_string()))
    }

    /// Any text URL (Import Codon Usage ▸ from URL).
    pub fn get_text(&self, url: &str) -> Result<String> {
        let host = url.split('/').nth(2).unwrap_or(url).to_owned();
        self.get(&host, url, &[])
    }

    /// An NCBI record as a GenBank / GenPept flat file (`efetch`).
    pub fn ncbi_fetch(&self, db: NcbiDb, accession: &str) -> Result<String> {
        let text = self.get(
            "NCBI",
            NCBI_EFETCH,
            &[
                ("db", db.name().into()),
                ("id", accession.trim().into()),
                ("rettype", db.rettype().into()),
                ("retmode", "text".into()),
                ("tool", self.tool.clone()),
                ("email", self.email.clone()),
            ],
        )?;
        if text.trim().is_empty() || text.contains("<ERROR>") || text.starts_with("Error") {
            return Err(NetError::Malformed(format!(
                "NCBI has no {} record {accession}",
                db.name()
            )));
        }
        Ok(text)
    }

    /// An ENA / EMBL record as an EMBL flat file.
    pub fn ena_fetch(&self, accession: &str) -> Result<String> {
        let url = format!(
            "{ENA_BROWSER}/embl/{}?download=true&lineLimit=0",
            accession.trim()
        );
        let text = self.get("ENA", &url, &[])?;
        if !text.starts_with("ID   ") {
            return Err(NetError::Malformed(format!(
                "ENA has no record {accession}"
            )));
        }
        Ok(text)
    }

    /// Submits a BLAST search (`CMD=Put`).
    pub fn blast_put(&self, req: &BlastRequest) -> Result<BlastJob> {
        let mut form: Vec<(&str, String)> = vec![
            ("CMD", "Put".into()),
            ("PROGRAM", req.program.clone()),
            ("QUERY", req.query.clone()),
            ("HITLIST_SIZE", req.hitlist_size.max(1).to_string()),
            ("tool", self.tool.clone()),
            ("email", self.email.clone()),
        ];
        match (&req.database, &req.subject) {
            (_, Some(subject)) => form.push(("SUBJECTS", subject.clone())),
            (Some(db), None) => form.push(("DATABASE", db.clone())),
            (None, None) => {
                return Err(NetError::Malformed(
                    "a BLAST search needs a database or a subject".into(),
                ));
            }
        }
        if req.program == "megablast" {
            form.retain(|(k, _)| *k != "PROGRAM");
            form.push(("PROGRAM", "blastn".into()));
            form.push(("MEGABLAST", "on".into()));
        }
        let endpoint = if req.subject.is_some() {
            BLAST_ALIGN
        } else {
            BLAST
        };
        let resp = self
            .http
            .post(endpoint)
            .form(&form)
            .send()
            .map_err(|e| NetError::Unreachable("NCBI BLAST".into(), e.to_string()))?;
        if !resp.status().is_success() {
            return Err(NetError::Status(
                "NCBI BLAST".into(),
                resp.status().as_u16(),
            ));
        }
        let text = resp
            .text()
            .map_err(|e| NetError::Unreachable("NCBI BLAST".into(), e.to_string()))?;
        let job = parse_put(&text)?;
        self.last_poll
            .lock()
            .unwrap()
            .insert(job.rid.clone(), Instant::now());
        Ok(job)
    }

    /// Checks a request (`CMD=Get&FORMAT_OBJECT=SearchInfo`), enforcing the
    /// one-poll-per-minute rule per RID.
    pub fn blast_status(&self, rid: &str) -> Result<BlastStatus> {
        {
            let last = self.last_poll.lock().unwrap();
            if let Some(t) = last.get(rid)
                && t.elapsed() < POLL_INTERVAL
            {
                return Err(NetError::TooSoon(
                    (POLL_INTERVAL - t.elapsed()).as_secs() + 1,
                ));
            }
        }
        let text = self.get(
            "NCBI BLAST",
            BLAST,
            &[
                ("CMD", "Get".into()),
                ("FORMAT_OBJECT", "SearchInfo".into()),
                ("RID", rid.to_owned()),
                ("tool", self.tool.clone()),
                ("email", self.email.clone()),
            ],
        )?;
        self.last_poll
            .lock()
            .unwrap()
            .insert(rid.to_owned(), Instant::now());
        Ok(parse_status(&text))
    }

    /// Seconds until `rid` may be polled again (0 = now).
    pub fn seconds_until_poll(&self, rid: &str) -> u64 {
        let last = self.last_poll.lock().unwrap();
        last.get(rid)
            .map(|t| POLL_INTERVAL.saturating_sub(t.elapsed()).as_secs())
            .unwrap_or(0)
    }

    /// Fetches a finished request's report (`FORMAT_TYPE=Text`, or
    /// `Tabular` / `XML`).
    pub fn blast_result(&self, rid: &str, format: &str) -> Result<String> {
        self.get(
            "NCBI BLAST",
            BLAST,
            &[
                ("CMD", "Get".into()),
                ("FORMAT_TYPE", format.into()),
                ("RID", rid.to_owned()),
                ("tool", self.tool.clone()),
                ("email", self.email.clone()),
            ],
        )
    }
}

/// Reads `RID` and `RTOE` out of the `QBlastInfo` comment of a Put answer.
pub fn parse_put(text: &str) -> Result<BlastJob> {
    let field = |name: &str| {
        text.lines().map(str::trim).find_map(|l| {
            l.strip_prefix(name)
                .and_then(|r| r.trim_start().strip_prefix('='))
                .map(|v| v.trim().to_owned())
        })
    };
    let rid = field("RID").filter(|r| !r.is_empty()).ok_or_else(|| {
        // NCBI reports problems as <p class="error">…</p> (or Message ID# lines).
        let msg = text
            .split("class=\"error\">")
            .nth(1)
            .map(|rest| {
                strip_tags(rest.split("</").next().unwrap_or(""))
                    .trim()
                    .to_owned()
            })
            .filter(|m| !m.is_empty())
            .or_else(|| {
                text.lines()
                    .find(|l| l.contains("Message ID"))
                    .map(|l| strip_tags(l).trim().to_owned())
            })
            .unwrap_or_else(|| "no request id in the BLAST answer".into());
        NetError::Malformed(format!("BLAST refused the request: {msg}"))
    })?;
    let rtoe = field("RTOE").and_then(|v| v.parse().ok()).unwrap_or(15);
    Ok(BlastJob { rid, rtoe })
}

/// Reads `Status=` / `ThereAreHits=` out of a SearchInfo answer.
pub fn parse_status(text: &str) -> BlastStatus {
    let field = |name: &str| {
        text.lines().map(str::trim).find_map(|l| {
            l.strip_prefix(name)
                .and_then(|r| r.strip_prefix('='))
                .map(|v| v.trim().to_owned())
        })
    };
    match field("Status").as_deref() {
        Some("WAITING") => BlastStatus::Waiting,
        Some("FAILED") => BlastStatus::Failed,
        Some("UNKNOWN") => BlastStatus::Unknown,
        Some("READY") => {
            if field("ThereAreHits").as_deref() == Some("yes") {
                BlastStatus::Ready
            } else {
                BlastStatus::NoHits
            }
        }
        _ => BlastStatus::Unknown,
    }
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_put_and_status_answers() {
        let put =
            "<html>\n<!--QBlastInfoBegin\n    RID = ABC123XYZ\n    RTOE = 23\nQBlastInfoEnd\n-->\n";
        let job = parse_put(put).unwrap();
        assert_eq!(
            job,
            BlastJob {
                rid: "ABC123XYZ".into(),
                rtoe: 23
            }
        );
        assert!(parse_put("<html><p class=\"error\">Message ID#24 Error: bad query</p>").is_err());
        assert_eq!(
            parse_status("QBlastInfoBegin\n\tStatus=WAITING\nQBlastInfoEnd"),
            BlastStatus::Waiting
        );
        assert_eq!(
            parse_status("\tStatus=READY\n\tThereAreHits=yes\n"),
            BlastStatus::Ready
        );
        assert_eq!(parse_status("\tStatus=READY\n"), BlastStatus::NoHits);
        assert_eq!(parse_status("garbage"), BlastStatus::Unknown);
    }

    #[test]
    fn poll_rate_is_enforced() {
        let c = Client::new("test", "test@example.org").unwrap();
        assert_eq!(c.seconds_until_poll("nope"), 0);
        c.last_poll
            .lock()
            .unwrap()
            .insert("r".into(), Instant::now());
        assert!(matches!(c.blast_status("r"), Err(NetError::TooSoon(_))));
        assert!(c.seconds_until_poll("r") >= 58);
    }

    // ---- network: run with `cargo test -p sc-net -- --ignored` ----

    #[test]
    #[ignore = "needs the network"]
    fn ncbi_fetches_pbr322() {
        let c = Client::new("SerialClon-test", "serialclon@example.org").unwrap();
        let gb = c.ncbi_fetch(NcbiDb::Nuccore, "J01749").unwrap();
        // The LOCUS name of pBR322 is SYNPBR322; the accession is on the ACCESSION line.
        assert!(gb.starts_with("LOCUS"), "{}", &gb[..gb.len().min(60)]);
        assert!(gb.contains("4361 bp") && gb.contains("J01749"));
        let gp = c.ncbi_fetch(NcbiDb::Protein, "P01857").unwrap();
        assert!(gp.starts_with("LOCUS"));
        assert!(c.ncbi_fetch(NcbiDb::Nuccore, "NOTANACCESSION999").is_err());
    }

    #[test]
    #[ignore = "needs the network"]
    fn ena_fetches_pbr322() {
        let c = Client::new("SerialClon-test", "serialclon@example.org").unwrap();
        let embl = c.ena_fetch("J01749").unwrap();
        assert!(embl.starts_with("ID   J01749"));
        assert!(embl.contains("4361 BP"));
    }

    #[test]
    #[ignore = "needs the network and takes a minute"]
    fn blast_two_sequences() {
        let c = Client::new("SerialClon-test", "serialclon@example.org").unwrap();
        let q = "ttctcatgtttgacagcttatcatcgataagctttaatgcggtagtttatcacagttaaattgctaacgcagtcaggcaccgtgtatgaaatctaacaatgcgctcatcgtcatcctcggcaccgtcaccctggatgctgtaggcataggcttggttatgccggtactgccgggcctcttgcgggatatc";
        let job = c
            .blast_put(&BlastRequest {
                program: "blastn".into(),
                database: None,
                query: q.into(),
                subject: Some(q.into()),
                hitlist_size: 5,
            })
            .unwrap();
        assert!(!job.rid.is_empty());
        std::thread::sleep(Duration::from_secs(job.rtoe.max(5)));
        c.last_poll.lock().unwrap().clear();
        let mut status = c.blast_status(&job.rid).unwrap();
        for _ in 0..5 {
            if status != BlastStatus::Waiting {
                break;
            }
            std::thread::sleep(POLL_INTERVAL);
            status = c.blast_status(&job.rid).unwrap();
        }
        assert_eq!(status, BlastStatus::Ready);
        let text = c.blast_result(&job.rid, "Text").unwrap();
        assert!(
            text.contains("Identities"),
            "{}",
            &text[..text.len().min(400)]
        );
    }

    #[test]
    #[ignore = "needs the network"]
    fn kazusa_page_is_fetched_as_text() {
        let c = Client::new("SerialClon-test", "serialclon@example.org").unwrap();
        let page = c
            .get_text(
                "https://www.kazusa.or.jp/codon/cgi-bin/showcodon.cgi?species=316407&aa=1&style=N",
            )
            .unwrap();
        assert!(page.contains("UUU"));
    }
}

/// A GitHub release, for the update check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    /// The tag, e.g. `v1.2.0`.
    pub tag_name: String,
    /// The release page.
    pub html_url: String,
}

impl Client {
    /// The latest published (non-draft, non-prerelease) release of a GitHub
    /// repository (`owner/name`), via the public REST API.
    pub fn latest_github_release(&self, repo: &str) -> Result<Release> {
        let text = self.get(
            "api.github.com",
            &format!("https://api.github.com/repos/{repo}/releases/latest"),
            &[],
        )?;
        serde_json::from_str::<Release>(&text)
            .map_err(|e| NetError::Malformed(format!("GitHub release: {e}")))
    }
}

/// Compares two `vMAJOR.MINOR.PATCH` strings; `true` when `candidate` is
/// newer than `current`. Pre-release suffixes are ignored.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['-', '+'])
            .next()
            .unwrap_or("")
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    }
    parts(candidate) > parts(current)
}

#[cfg(test)]
mod version_tests {
    use super::is_newer;

    #[test]
    fn version_comparison() {
        assert!(is_newer("v1.0.1", "1.0.0"));
        assert!(is_newer("v1.1.0", "v1.0.9"));
        assert!(!is_newer("v1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.9", "1.0.0"));
        assert!(is_newer("v2.0.0-beta.1", "1.9.0"));
    }
}
