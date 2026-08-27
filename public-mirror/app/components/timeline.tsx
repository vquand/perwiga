import Image from "next/image";
import type { CSSProperties } from "react";
import { useRef, useState } from "react";
import type { Event } from "../types";

type TimelineProps = { events: Event[] };
type EventState = "upcoming" | "ongoing" | "past";
type TimelineEvent = {
  event: Event;
  state: EventState;
  left: number;
  width: number;
  row: number;
};
type TooltipPosition = { top: number; left: number };

const DAY_MS = 24 * 60 * 60 * 1000;

function parseDate(value?: string) {
  if (!value) return undefined;
  const timestamp = new Date(value).getTime();
  return Number.isNaN(timestamp) ? undefined : timestamp;
}

function formatDate(value?: string) {
  if (!value) return "Open-ended";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" }).format(date);
}

function formatAxisDate(timestamp: number, rangeDays: number) {
  return new Intl.DateTimeFormat(undefined, rangeDays > 365 ? { month: "short", year: "numeric" } : { month: "short", day: "numeric" }).format(new Date(timestamp));
}

function eventState(event: Event, now = Date.now()): EventState {
  const start = parseDate(event.starts_at) ?? now;
  const end = parseDate(event.ends_at) ?? Number.POSITIVE_INFINITY;
  if (now < start) return "upcoming";
  if (now > end) return "past";
  return "ongoing";
}

function groupEvents(events: Event[]) {
  return events.reduce<Record<string, Event[]>>((groups, event) => {
    const key = event.event_type || "Other";
    groups[key] ||= [];
    groups[key].push(event);
    return groups;
  }, {});
}

function axisStepDays(rangeDays: number) {
  if (rangeDays <= 45) return 7;
  if (rangeDays <= 120) return 14;
  if (rangeDays <= 365) return 30;
  return 90;
}

function createTicks(startMs: number, endMs: number) {
  const rangeDays = (endMs - startMs) / DAY_MS;
  const stepMs = axisStepDays(rangeDays) * DAY_MS;
  const ticks: number[] = [];
  for (let timestamp = startMs; timestamp <= endMs; timestamp += stepMs) ticks.push(timestamp);
  if (ticks[ticks.length - 1] !== endMs) ticks.push(endMs);
  return ticks;
}

function layoutLane(events: Event[], axisStart: number, axisEnd: number): TimelineEvent[] {
  const rangeMs = Math.max(axisEnd - axisStart, DAY_MS);
  const rowEnds: number[] = [];
  const now = Date.now();

  return [...events]
    .sort((left, right) => (parseDate(left.starts_at) ?? 0) - (parseDate(right.starts_at) ?? 0))
    .map((event) => {
      const startMs = Math.max(parseDate(event.starts_at) ?? axisStart, axisStart);
      const declaredEnd = parseDate(event.ends_at);
      const endMs = Math.max(declaredEnd ?? axisEnd, startMs + 1);
      const row = rowEnds.findIndex((rowEnd) => rowEnd <= startMs);
      const assignedRow = row === -1 ? rowEnds.length : row;
      rowEnds[assignedRow] = endMs;
      const visibleEnd = Math.min(endMs, axisEnd);
      return {
        event,
        state: eventState(event, now),
        left: ((startMs - axisStart) / rangeMs) * 100,
        width: Math.max(((visibleEnd - startMs) / rangeMs) * 100, 1.4),
        row: assignedRow,
      };
    });
}

function styleVars(values: Record<string, number | string>): CSSProperties {
  return values as CSSProperties;
}

function eventSummary(item: TimelineEvent) {
  const { event } = item;
  return `${event.title}. ${formatDate(event.starts_at)}${event.ends_at ? ` to ${formatDate(event.ends_at)}` : ". End not announced"}`;
}

function tooltipId(event: Event) {
  return `timeline-detail-${event.id.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
}

export function Timeline({ events }: TimelineProps) {
  const [activeTooltipId, setActiveTooltipId] = useState<string | null>(null);
  const [tooltipPosition, setTooltipPosition] = useState<TooltipPosition | null>(null);
  const closeTooltipTimeout = useRef<number | undefined>(undefined);

  if (!events.length) {
    return <div className="empty-state"><span className="empty-glyph" aria-hidden="true">⌁</span><h3>No events yet</h3><p>There are no events to show for this work.</p></div>;
  }

  const validStarts = events.map((event) => parseDate(event.starts_at)).filter((timestamp): timestamp is number => timestamp !== undefined);
  const validEnds = events.map((event) => parseDate(event.ends_at)).filter((timestamp): timestamp is number => timestamp !== undefined);
  const axisStart = Math.min(...validStarts);
  const latestStart = Math.max(...validStarts);
  const latestEnd = Math.max(...validEnds, latestStart + 14 * DAY_MS);
  const axisEnd = Math.max(latestEnd, axisStart + DAY_MS);
  const rangeDays = Math.ceil((axisEnd - axisStart) / DAY_MS);
  const ticks = createTicks(axisStart, axisEnd);
  const groups = Object.entries(groupEvents(events)).map(([type, items]) => ({ type, items: layoutLane(items, axisStart, axisEnd) }));

  function cancelTooltipClose() {
    if (closeTooltipTimeout.current !== undefined) window.clearTimeout(closeTooltipTimeout.current);
  }

  function closeTooltip() {
    cancelTooltipClose();
    setActiveTooltipId(null);
    setTooltipPosition(null);
  }

  function closeTooltipSoon() {
    cancelTooltipClose();
    closeTooltipTimeout.current = window.setTimeout(closeTooltip, 140);
  }

  function revealTooltip(item: TimelineEvent, button: HTMLButtonElement) {
    cancelTooltipClose();
    const rect = button.getBoundingClientRect();
    const tooltipWidth = Math.min(416, Math.max(16, window.innerWidth - 32));
    const tooltipHeight = 240;
    const left = Math.min(Math.max(16, rect.left), Math.max(16, window.innerWidth - tooltipWidth - 16));
    const top = rect.bottom + 12 + tooltipHeight <= window.innerHeight - 16 ? rect.bottom + 12 : Math.max(16, rect.top - tooltipHeight - 12);
    setTooltipPosition({ top, left });
    setActiveTooltipId(item.event.id);
  }

  return (
    <div className="timeline-stack" aria-label="Event timeline">
      <div className="timeline-key" aria-label="Timeline legend">
        <span className="timeline-key-item"><i className="timeline-key-swatch is-past" aria-hidden="true" />Past</span>
        <span className="timeline-key-item"><i className="timeline-key-swatch is-ongoing" aria-hidden="true" />Ongoing</span>
        <span className="timeline-key-item"><i className="timeline-key-swatch is-upcoming" aria-hidden="true" />Upcoming</span>
        <span className="timeline-range">{formatDate(new Date(axisStart).toISOString())} — {formatDate(new Date(axisEnd).toISOString())}</span>
      </div>

      <div className="timeline-scroll">
        <div className="timeline-canvas">
          <div className="timeline-axis" aria-hidden="true">
            <div className="timeline-axis-label"><span className="eyebrow">Schedule</span><strong>Event type</strong></div>
            <div className="timeline-axis-track">
              {ticks.map((timestamp) => <span className="timeline-axis-tick" key={timestamp} style={styleVars({ "--tick-position": ((timestamp - axisStart) / (axisEnd - axisStart)) * 100 })}><span>{formatAxisDate(timestamp, rangeDays)}</span></span>)}
            </div>
          </div>

          {groups.map(({ type, items }) => (
            <section className="timeline-lane" key={type} aria-label={`${type} event swimlane`}>
              <header>
                <span className="lane-marker" aria-hidden="true" />
                <div><p className="eyebrow">Swimlane</p><h3>{type}</h3></div>
                <span className="lane-count">{items.length} {items.length === 1 ? "event" : "events"}</span>
              </header>
              <div className="timeline-lane-body">
                <div className="timeline-lane-track" style={styleVars({ "--lane-height": `${Math.max(items.reduce((largest, item) => Math.max(largest, item.row), 0) + 1, 2) * 2.4 + 1.2}rem` })}>
                  <div className="timeline-grid-lines" aria-hidden="true">
                    {ticks.map((timestamp) => <span key={timestamp} style={styleVars({ "--tick-position": ((timestamp - axisStart) / (axisEnd - axisStart)) * 100 })} />)}
                  </div>
                  {items.map((item) => {
                    const { event } = item;
                    const detailId = tooltipId(event);
                    return (
                      <div className={`timeline-event-anchor is-${item.state}`} key={event.id} style={styleVars({ "--event-left": item.left, "--event-width": item.width, "--event-row": item.row })}>
                        <button className={`timeline-bar is-${item.state}${event.ends_at ? "" : " is-open-ended"}`} type="button" aria-label={eventSummary(item)} aria-describedby={activeTooltipId === event.id ? detailId : undefined} onMouseEnter={(mouseEvent) => revealTooltip(item, mouseEvent.currentTarget)} onMouseLeave={closeTooltipSoon} onFocus={(focusEvent) => revealTooltip(item, focusEvent.currentTarget)} onBlur={(focusEvent) => { const nextFocus = focusEvent.relatedTarget; if (nextFocus instanceof Node && focusEvent.currentTarget.parentElement?.contains(nextFocus)) return; closeTooltipSoon(); }}>
                          <span className="timeline-bar-title">{event.title}</span>
                        </button>
                        {activeTooltipId === event.id && tooltipPosition ? <div className="timeline-event-tooltip" id={detailId} role="tooltip" style={styleVars({ "--tooltip-top": tooltipPosition.top, "--tooltip-left": tooltipPosition.left })} onMouseEnter={cancelTooltipClose} onMouseLeave={closeTooltipSoon}>
                          <div className="event-date"><strong>{formatDate(event.starts_at)}</strong><span>{event.ends_at ? `to ${formatDate(event.ends_at)}` : "End not announced"}</span></div>
                          <div className="event-copy">
                            <div className="event-title-line"><h4>{event.title}</h4><span className="state-badge">{item.state}</span></div>
                            {(event.patch_start || event.patch_end) && <p className="patch-label">Patch {event.patch_start}{event.patch_end && event.patch_end !== event.patch_start ? `–${event.patch_end}` : ""}</p>}
                            {event.presentation?.featured_entities.length ? (
                              <div className="featured-strip" aria-label="Featured entities">
                                {event.presentation.featured_entities.map((featured) => (
                                  <span className="featured-entity" key={featured.display_name}>
                                    <Image src={featured.thumbnail_url} alt="" width={32} height={32} loading="lazy" unoptimized />
                                    <span>{featured.display_name}</span>
                                    {featured.previous_event_gap && <small>{featured.previous_event_gap.days}d gap</small>}
                                  </span>
                                ))}
                              </div>
                            ) : null}
                            {event.source_url && <a className="source-link" href={event.source_url} target="_blank" rel="noreferrer">View source ↗</a>}
                          </div>
                        </div> : null}
                      </div>
                    );
                  })}
                </div>
              </div>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
