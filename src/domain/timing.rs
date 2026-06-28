#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HttpTiming {
    pub prepare_request_ms: u128,
    pub server_wait_ms: u128,
    pub read_body_ms: u128,
    pub parse_response_ms: u128,
}

impl HttpTiming {
    pub fn total_ms(self) -> u128 {
        self.prepare_request_ms
            + self.server_wait_ms
            + self.read_body_ms
            + self.parse_response_ms
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestTimingBreakdown {
    pub pre_request_script_ms: u128,
    pub http: HttpTiming,
    pub post_response_script_ms: u128,
}

impl RequestTimingBreakdown {
    pub fn total_ms(self) -> u128 {
        self.pre_request_script_ms + self.http.total_ms() + self.post_response_script_ms
    }

    pub fn visible_spans(self) -> Vec<(&'static str, u128)> {
        let mut spans = Vec::new();
        if self.pre_request_script_ms > 0 {
            spans.push(("Pre-request script", self.pre_request_script_ms));
        }
        if self.http.prepare_request_ms > 0 {
            spans.push(("Prepare request", self.http.prepare_request_ms));
        }
        if self.http.server_wait_ms > 0 {
            spans.push(("Server wait", self.http.server_wait_ms));
        }
        if self.http.read_body_ms > 0 {
            spans.push(("Read body", self.http.read_body_ms));
        }
        if self.http.parse_response_ms > 0 {
            spans.push(("Parse response", self.http.parse_response_ms));
        }
        if self.post_response_script_ms > 0 {
            spans.push(("Post-response script", self.post_response_script_ms));
        }
        spans
    }
}
