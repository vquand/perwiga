import Image from "next/image";
import type { Event } from "../types";

type TimelineProps = { events: Event[] };

function formatDate(value?: string) {
  if (!value) return "Open-ended";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" }).format(date);
}

function eventState(event: Event) {
  const now = Date.now();
  const start = new Date(event.starts_at).getTime();
  const end = event.ends_at ? new Date(event.ends_at).getTime() : Number.POSITIVE_INFINITY;
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

export function Timeline({ events }: TimelineProps) {
  if (!events.length) {
    return <div className="empty-state"><span className="empty-glyph" aria-hidden="true">⌁</span><h3>No public events yet</h3><p>Events with verified public sources will appear here.</p></div>;
  }

  const groups = groupEvents(events);
  return (
    <div className="timeline-stack" aria-label="Public event timeline">
      {Object.entries(groups).map(([type, items]) => (
        <section className="timeline-lane" key={type}>
          <header>
            <span className="lane-marker" aria-hidden="true" />
            <div><p className="eyebrow">Swimlane</p><h3>{type}</h3></div>
            <span className="lane-count">{items.length} {items.length === 1 ? "event" : "events"}</span>
          </header>
          <div className="timeline-events">
            {items.map((event) => {
              const state = eventState(event);
              return (
                <article className={`timeline-event is-${state}`} key={event.id}>
                  <div className="event-date"><strong>{formatDate(event.starts_at)}</strong><span>{event.ends_at ? `to ${formatDate(event.ends_at)}` : "End not announced"}</span></div>
                  <div className="event-copy">
                    <div className="event-title-line"><h4>{event.title}</h4><span className="state-badge">{state}</span></div>
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
                </article>
              );
            })}
          </div>
        </section>
      ))}
    </div>
  );
}
