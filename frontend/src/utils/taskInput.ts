import type { Priority } from "../api/types";

export interface ProjectRef {
  id: string;
  name: string;
}

export interface QuickAddResult {
  title: string;
  dueDate: Date | null;
  priority: Priority | null;
  projectId: string | null;
  projectName: string | null;
}

const WEEKDAYS: Record<string, number> = {
  sun: 0,
  sunday: 0,
  mon: 1,
  monday: 1,
  tue: 2,
  tues: 2,
  tuesday: 2,
  wed: 3,
  weds: 3,
  wednesday: 3,
  thu: 4,
  thur: 4,
  thurs: 4,
  thursday: 4,
  fri: 5,
  friday: 5,
  sat: 6,
  saturday: 6,
};

const MONTHS: Record<string, number> = {
  jan: 0,
  january: 0,
  feb: 1,
  february: 1,
  mar: 2,
  march: 2,
  apr: 3,
  april: 3,
  may: 4,
  jun: 5,
  june: 5,
  jul: 6,
  july: 6,
  aug: 7,
  august: 7,
  sep: 8,
  sept: 8,
  september: 8,
  oct: 9,
  october: 9,
  nov: 10,
  november: 10,
  dec: 11,
  december: 11,
};

function atTime(base: Date, hours: number, minutes: number): Date {
  const d = new Date(base);
  d.setHours(hours, minutes, 0, 0);
  return d;
}

/** Parse "5pm", "17:00", "9", "9:30am" → [hours, minutes] or null. */
function parseTimeToken(token: string): [number, number] | null {
  const m = token.trim().toLowerCase().match(/^(\d{1,2})(?::(\d{2}))?\s*(am|pm)?$/);
  if (!m) return null;
  let h = parseInt(m[1], 10);
  const min = m[2] ? parseInt(m[2], 10) : 0;
  const meridiem = m[3];
  if (meridiem) {
    if (h < 1 || h > 12 || min > 59) return null;
    if (meridiem === "pm" && h !== 12) h += 12;
    if (meridiem === "am" && h === 12) h = 0;
  } else {
    if (h > 23 || min > 59) return null;
    // Bare "9" (no minutes, no am/pm) is ambiguous with day numbers —
    // only accept it as a time when it looks like one (handled by caller).
  }
  return [h, min];
}

/**
 * Parse a natural-language date expression.
 *
 * Accepted (case-insensitive):
 *   today | tomorrow | tmr | day after tomorrow | next week
 *   mon..sun (next occurrence, strictly in the future)
 *   in N days|weeks|hours|mins
 *   YYYY-MM-DD [HH:MM [am|pm]]  (also YYYY-MM-DDTHH:MM passthrough)
 *   sep 30 | 30 sep [HH:MM]
 *   HH:MM / 5pm (today, or tomorrow if that time already passed)
 *   each optionally followed by "at <time>"
 *
 * A bare date defaults to 09:00 local. Returns null when unparseable.
 */
export function parseNaturalDate(raw: string, now = new Date()): Date | null {
  let s = raw.trim().toLowerCase().replace(/\s+/g, " ");
  if (!s) return null;

  // Split off a trailing "at <time>" — e.g. "tomorrow at 5pm".
  let timePart: string | null = null;
  const atMatch = s.match(/^(.*)\s+at\s+(\d{1,2}(?::\d{2})?\s*(?:am|pm)?)$/);
  if (atMatch) {
    s = atMatch[1].trim();
    timePart = atMatch[2].trim();
  } else {
    // Or a bare trailing time token: "fri 15:00", "tomorrow 9am".
    const parts = s.split(" ");
    if (parts.length >= 2) {
      const maybeTime = parseTimeToken(parts[parts.length - 1]);
      // Only treat a bare hour ("9") as time when the rest is a known date word.
      const rest = parts.slice(0, -1).join(" ");
      const restIsDateWord =
        /^(today|tod|tomorrow|tmr|tmrw|day after tomorrow|next week)$/.test(rest) ||
        WEEKDAYS[rest] !== undefined;
      const last = parts[parts.length - 1];
      const hasExplicitTime = last.includes(":") || /am|pm/.test(last);
      if (maybeTime && (hasExplicitTime || restIsDateWord)) {
        timePart = parts[parts.length - 1];
        s = rest;
      }
    }
  }

  const applyTime = (d: Date): Date => {
    if (!timePart) {
      let r = atTime(d, 9, 0);
      // Bare date resolving to earlier today (e.g. "today" after 09:00):
      // keep it due today, end of workday, instead of instantly overdue.
      if (r.getTime() <= now.getTime() && r.toDateString() === now.toDateString()) {
        r = atTime(d, 17, 0);
      }
      return r;
    }
    const t = parseTimeToken(timePart);
    if (!t) return atTime(d, 9, 0);
    return atTime(d, t[0], t[1]);
  };

  // Relative words.
  if (s === "today" || s === "tod") return applyTime(new Date(now));
  if (s === "tomorrow" || s === "tmr" || s === "tmrw") {
    const d = new Date(now);
    d.setDate(d.getDate() + 1);
    return applyTime(d);
  }
  if (s === "day after tomorrow") {
    const d = new Date(now);
    d.setDate(d.getDate() + 2);
    return applyTime(d);
  }
  if (s === "next week") {
    const d = new Date(now);
    d.setDate(d.getDate() + 7);
    return applyTime(d);
  }

  // Weekday → next occurrence strictly in the future.
  if (WEEKDAYS[s] !== undefined) {
    const target = WEEKDAYS[s];
    const d = new Date(now);
    let delta = (target - d.getDay() + 7) % 7;
    if (delta === 0) delta = 7;
    d.setDate(d.getDate() + delta);
    return applyTime(d);
  }

  // "in N <unit>".
  const inMatch = s.match(/^in\s+(\d+)\s*(min|mins|minute|minutes|hour|hours|day|days|week|weeks)$/);
  if (inMatch) {
    const n = parseInt(inMatch[1], 10);
    const unit = inMatch[2];
    const d = new Date(now);
    if (unit.startsWith("min")) d.setMinutes(d.getMinutes() + n);
    else if (unit.startsWith("hour")) d.setHours(d.getHours() + n);
    else if (unit.startsWith("day")) d.setDate(d.getDate() + n);
    else d.setDate(d.getDate() + n * 7);
    if (!timePart && (unit.startsWith("day") || unit.startsWith("week"))) {
      return atTime(d, 9, 0);
    }
    return d;
  }

  // ISO date, optional time (space or T separator).
  const iso = s.match(/^(\d{4})-(\d{2})-(\d{2})(?:[t\s](\d{1,2})(?::(\d{2}))?)?$/);
  if (iso) {
    const y = parseInt(iso[1], 10);
    const mo = parseInt(iso[2], 10) - 1;
    const day = parseInt(iso[3], 10);
    if (mo < 0 || mo > 11 || day < 1 || day > 31) return null;
    if (iso[4] !== undefined) {
      return new Date(y, mo, day, parseInt(iso[4], 10), iso[5] ? parseInt(iso[5], 10) : 0, 0, 0);
    }
    return applyTime(new Date(y, mo, day, 12, 0, 0, 0));
  }

  // "sep 30" / "30 sep" (current year, or next year if already passed).
  const monthFirst = s.match(/^([a-z]+)\s+(\d{1,2})$/);
  const dayFirst = s.match(/^(\d{1,2})\s+([a-z]+)$/);
  const md = monthFirst ?? (dayFirst ? [dayFirst[0], dayFirst[2], dayFirst[1]] : null);
  if (md) {
    const mo = MONTHS[md[1]];
    const day = parseInt(md[2], 10);
    if (mo === undefined || day < 1 || day > 31) return null;
    let d = new Date(now.getFullYear(), mo, day, 12, 0, 0, 0);
    if (d.getTime() < now.getTime() - 24 * 3600 * 1000) {
      d = new Date(now.getFullYear() + 1, mo, day, 12, 0, 0, 0);
    }
    return applyTime(d);
  }

  // Bare time → today, or tomorrow if already passed.
  const bareTime = parseTimeToken(s);
  if (bareTime && (s.includes(":") || /am|pm/.test(s))) {
    let d = atTime(now, bareTime[0], bareTime[1]);
    if (d.getTime() <= now.getTime()) {
      d = new Date(d);
      d.setDate(d.getDate() + 1);
    }
    return d;
  }

  return null;
}

const PRIORITY_TOKENS: Record<string, Priority> = {
  "!low": "low",
  "!medium": "medium",
  "!med": "medium",
  "!high": "high",
  "!urgent": "urgent",
  p4: "low",
  p3: "medium",
  p2: "high",
  p1: "urgent",
};

/** Match a typed project name against the project list (exact → prefix → substring). */
export function matchProject(name: string, projects: ProjectRef[]): ProjectRef | null {
  return findProject(name, projects);
}

function findProject(name: string, projects: ProjectRef[]): ProjectRef | null {  const q = name.toLowerCase();
  return (
    projects.find((p) => p.name.toLowerCase() === q) ??
    projects.find((p) => p.name.toLowerCase().startsWith(q)) ??
    projects.find((p) => p.name.toLowerCase().includes(q)) ??
    null
  );
}

/**
 * Parse a quick-add string like:
 *   "Buy milk tomorrow !high #home"
 *
 * Rules:
 * - `#project` (word, matched case-insensitively against project names)
 * - `!low|!medium|!high|!urgent` or `p1..p4` (Todoist-style) for priority
 * - a natural date expression at the END of the string, or `due:<expr>`
 * - everything left over is the title
 */
export function parseQuickAdd(input: string, projects: ProjectRef[] = []): QuickAddResult {
  let priority: Priority | null = null;
  let projectId: string | null = null;
  let projectName: string | null = null;

  const tokens = input.split(/\s+/).filter(Boolean);
  const kept: string[] = [];

  for (const tok of tokens) {
    const low = tok.toLowerCase();
    if (PRIORITY_TOKENS[low] && !priority) {
      priority = PRIORITY_TOKENS[low];
      continue;
    }
    if (tok.startsWith("#") && tok.length > 1 && projectId === null) {
      const found = findProject(tok.slice(1), projects);
      if (found) {
        projectId = found.id;
        projectName = found.name;
        continue;
      }
    }
    kept.push(tok);
  }

  // Explicit `due:<expr>` anywhere in the string wins over trailing dates.
  let dueDate: Date | null = null;
  const dueIdx = kept.findIndex((t) => t.toLowerCase().startsWith("due:"));
  if (dueIdx !== -1) {
    dueDate = parseNaturalDate(kept[dueIdx].slice(4));
    if (dueDate) kept.splice(dueIdx, 1);
    else kept[dueIdx] = kept[dueIdx].slice(4); // not a date → treat as plain word
  }

  if (!dueDate) {
    // Longest trailing match (up to 4 tokens) that parses as a date.
    const maxN = Math.min(4, kept.length - 1); // keep at least 1 word for the title
    for (let n = maxN; n >= 1; n--) {
      const candidate = kept.slice(kept.length - n).join(" ");
      const parsed = parseNaturalDate(candidate);
      if (parsed) {
        dueDate = parsed;
        kept.splice(kept.length - n, n);
        break;
      }
    }
  }

  return { title: kept.join(" ").trim(), dueDate, priority, projectId, projectName };
}

/** "Tomorrow, 09:00" style preview for a parsed date. */
export function formatDuePreview(d: Date, now = new Date()): string {
  const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const day = new Date(d);
  day.setHours(0, 0, 0, 0);
  const today = new Date(now);
  today.setHours(0, 0, 0, 0);
  const diffDays = Math.round((day.getTime() - today.getTime()) / 86400000);
  if (diffDays === 0) return `Today, ${time}`;
  if (diffDays === 1) return `Tomorrow, ${time}`;
  return `${d.toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" })}, ${time}`;
}
