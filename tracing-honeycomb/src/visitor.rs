use chrono::{DateTime, Utc};
use libhoney::{json, Value};
use std::collections::HashMap;
use std::fmt;
use tracing::field::{Field, Visit};
use tracing_distributed::{Event, Span};

use crate::{SpanId, TraceId};

// Visitor that builds honeycomb-compatible values from tracing fields.
#[derive(Default, Debug)]
#[doc(hidden)]
pub struct HoneycombVisitor(pub(crate) HashMap<String, Value>);

// reserved field names (TODO: document)
static RESERVED_WORDS: [&str; 9] = [
    "trace.span_id",
    "trace.trace_id",
    "trace.parent_id",
    "service_name",
    "level",
    "Timestamp",
    "name",
    "target",
    "duration_ms",
];

impl Visit for HoneycombVisitor {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0
            .insert(mk_field_name(field.name().to_string()), json!(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0
            .insert(mk_field_name(field.name().to_string()), json!(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0
            .insert(mk_field_name(field.name().to_string()), json!(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0
            .insert(mk_field_name(field.name().to_string()), json!(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let s = format!("{:?}", value);
        self.0
            .insert(mk_field_name(field.name().to_string()), json!(s));
    }
}

fn mk_field_name(s: String) -> String {
    // TODO: do another pass, optimize for efficiency (lazy static set?)
    if RESERVED_WORDS.contains(&&s[..]) {
        format!("tracing.{}", s)
    } else {
        s
    }
}

pub(crate) fn event_to_values(
    event: Event<HoneycombVisitor, SpanId, TraceId>,
) -> HashMap<String, libhoney::Value> {
    let mut values = event.values.0;

    values.insert(
        // magic honeycomb string (trace.trace_id)
        "trace.trace_id".to_string(),
        // using explicit trace id passed in from ctx (req'd for lazy eval)
        json!(event.trace_id.to_string()),
    );

    values.insert(
        // magic honeycomb string (trace.parent_id)
        "trace.parent_id".to_string(),
        event
            .parent_id
            .map(|pid| json!(format!("span-{}", pid.to_string())))
            .unwrap_or(json!(null)),
    );

    // magic honeycomb string (service_name)
    values.insert("service_name".to_string(), json!(event.service_name));

    values.insert(
        "level".to_string(),
        json!(format!("{}", event.meta.level())),
    );

    let initialized_at: DateTime<Utc> = event.initialized_at.into();
    values.insert("Timestamp".to_string(), json!(initialized_at.to_rfc3339()));

    // not honeycomb-special but tracing-provided
    values.insert("name".to_string(), json!(event.meta.name()));
    values.insert("target".to_string(), json!(event.meta.target()));

    values
}

pub(crate) fn span_to_values(
    span: Span<HoneycombVisitor, SpanId, TraceId>,
) -> HashMap<String, libhoney::Value> {
    let mut values = span.values.0;

    values.insert(
        // magic honeycomb string (trace.span_id)
        "trace.span_id".to_string(),
        json!(format!("span-{}", span.id.to_string())),
    );

    values.insert(
        // magic honeycomb string (trace.trace_id)
        "trace.trace_id".to_string(),
        // using explicit trace id passed in from ctx (req'd for lazy eval)
        json!(span.trace_id.to_string()),
    );

    values.insert(
        // magic honeycomb string (trace.parent_id)
        "trace.parent_id".to_string(),
        span.parent_id
            .map(|pid| json!(format!("span-{}", pid.to_string())))
            .unwrap_or(json!(null)),
    );

    // magic honeycomb string (service_name)
    values.insert("service_name".to_string(), json!(span.service_name));

    values.insert("level".to_string(), json!(format!("{}", span.meta.level())));

    let initialized_at: DateTime<Utc> = span.initialized_at.into();
    values.insert("Timestamp".to_string(), json!(initialized_at.to_rfc3339()));

    // not honeycomb-special but tracing-provided
    values.insert("name".to_string(), json!(span.meta.name()));
    values.insert("target".to_string(), json!(span.meta.target()));

    match span.completed_at.duration_since(span.initialized_at) {
        Ok(d) => {
            // honeycomb-special (I think, todo: get full list of known values)
            values.insert("duration_ms".to_string(), json!(d.as_millis() as u64));
        }
        Err(e) => {
            eprintln!("error comparing system times in tracing-honeycomg, indicates possible clock skew: {:?}", e);
        }
    }

    values
}
