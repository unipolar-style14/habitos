# HabitOS PRD

## AI-Powered Terminal Operating System for Personal Execution

### Version

1.0

### Status

Product Requirements Document

### Owner

Founder

---

# 1. Vision

HabitOS is a local-first AI-powered terminal application that helps users plan, execute, reflect, and improve their lives.

Unlike traditional habit trackers that only record completion, HabitOS acts as an AI Chief of Staff.

It combines:

* Habit tracking
* Goal management
* Daily planning
* Focus tracking
* Journaling
* AI coaching
* Long-term personal memory

into a single terminal-first experience.

The product should feel like:

> "An operating system for running your life."

---

# 2. Problem Statement

Current tools are fragmented:

* Todo apps manage tasks
* Habit apps track streaks
* Calendars manage schedules
* Journals capture thoughts
* AI tools provide isolated advice

Users constantly switch between systems.

There is no unified system that:

* Understands goals
* Tracks habits
* Learns behavior
* Provides accountability
* Operates locally
* Works from the terminal

HabitOS solves this problem.

---

# 3. Target Users

## Primary

Developers

Engineers

Founders

Indie Hackers

Technical Professionals

## Secondary

Students

Researchers

Writers

Knowledge Workers

---

# 4. Product Principles

### Local First

All data remains on the user's machine.

### Offline First

Core functionality works without internet.

### AI Native

AI is embedded into workflows.

### Terminal First

CLI is the primary interface.

### Privacy Focused

No telemetry.

No cloud dependency.

No tracking.

---

# 5. Core Features

## 5.1 Habit Tracking

Users can create habits.

Examples:

* Workout
* Read
* Meditate
* Walk
* Study

### Requirements

Track:

* Daily completion
* Weekly completion
* Streaks
* Longest streak
* Missed days

Commands:

```bash
habitos habit add "Workout"

habitos habit done workout

habitos habit skip workout

habitos habit stats
```

---

## 5.2 Goal Management

Goals represent long-term outcomes.

Examples:

* Launch HostOps
* Lose 10kg
* Publish 20 articles

### Requirements

Track:

* Progress
* Milestones
* Deadlines
* Priority

Commands:

```bash
habitos goal add "Launch HostOps"

habitos goal progress

habitos goal complete
```

---

## 5.3 Daily Planning

Users begin each day with a plan.

Command:

```bash
habitos plan
```

Inputs:

* Open goals
* Incomplete tasks
* Habit status
* Calendar events

Output:

* Prioritized agenda
* Focus blocks
* Top priorities

---

## 5.4 Focus Sessions

Track deep work.

Commands:

```bash
habitos focus start

habitos focus stop
```

Track:

* Start time
* End time
* Duration
* Project
* Notes

Metrics:

* Daily focus hours
* Weekly focus hours
* Average session length

---

## 5.5 Journaling

Users maintain a daily journal.

Commands:

```bash
habitos journal new

habitos journal today

habitos journal search
```

Track:

* Thoughts
* Learnings
* Wins
* Challenges

---

## 5.6 Reflection System

Command:

```bash
habitos reflect
```

Questions:

* What went well?
* What did not?
* What did you learn?
* What is tomorrow's priority?

Output:

AI-generated summary.

---

## 5.7 Review Engine

Daily Review

Weekly Review

Monthly Review

Quarterly Review

Metrics:

* Goal progress
* Habit consistency
* Focus trends
* Productivity score

---

# 6. AI System

## Overview

AI serves as:

* Coach
* Planner
* Analyst
* Accountability Partner

---

## Supported Models

### Primary

Gemma

### Secondary

Qwen

Llama

Mistral

DeepSeek

---

## Runtime

Ollama

OpenAI-compatible APIs

LM Studio

Local inference servers

---

# 7. AI Features

## Daily Planner

Command:

```bash
habitos plan
```

Produces:

* Daily priorities
* Suggested schedule
* Risk warnings

---

## Coach

Command:

```bash
habitos coach
```

Analyzes:

* Habits
* Goals
* Focus sessions
* Journals

Provides:

* Advice
* Patterns
* Recommendations

---

## Weekly Review

Command:

```bash
habitos review week
```

Provides:

* Wins
* Failures
* Trends
* Next actions

---

## Life Insights

Command:

```bash
habitos insights
```

Examples:

* Most productive hours
* Habit correlations
* Goal velocity
* Burnout indicators

---

# 8. Long-Term Memory

Store:

* Journal entries
* Reviews
* Goals
* Reflections
* AI summaries

Enable semantic search.

Command:

```bash
habitos ask
```

Example:

```text
What was I focused on in March?

What goals did I abandon?

How consistent was my workout habit?
```

---

# 9. Architecture

## Components

### CLI

User interface.

### Application Layer

Business logic.

### Domain Layer

Entities and rules.

### Persistence Layer

SQLite.

### AI Layer

LLM integrations.

### Analytics Layer

Reports and insights.

### Plugin Layer

Optional integrations.

---

# 10. Technical Stack

Language:

Rust

CLI:

clap

Storage:

SQLite

Database Access:

sqlx

Serialization:

serde

Configuration:

toml

AI:

Ollama

Embedding Storage:

SQLite + Vector Extension

Testing:

cargo test

---

# 11. Database Schema

Tables:

users

habits

habit_logs

goals

goal_milestones

focus_sessions

journal_entries

daily_reviews

weekly_reviews

monthly_reviews

ai_memories

settings

event_log

---

# 12. Plugin System

Support plugins.

Examples:

Git Plugin

Calendar Plugin

Obsidian Plugin

Markdown Export Plugin

CSV Export Plugin

Notification Plugin

Telegram Plugin

OpenClaw Plugin

---

# 13. Reporting

## Daily

Habit completion

Focus hours

Journal summary

---

## Weekly

Goal progress

Habit consistency

AI insights

---

## Monthly

Performance report

Productivity score

Improvement areas

---

# 14. Security

Local-only database.

Encrypted secrets.

No analytics.

No telemetry.

No third-party tracking.

No cloud dependency.

---

# 15. Success Metrics

### User Metrics

Daily active usage

Habit completion rate

Review completion rate

Focus hours tracked

### Product Metrics

Startup time < 100ms

AI response < 5s

SQLite queries < 50ms

Memory usage < 150MB

---

# 16. Future Roadmap

V2

* TUI dashboard
* AI scheduling
* Calendar sync
* Mobile companion

V3

* Voice assistant
* Agent workflows
* Automated task execution
* Personal knowledge graph

V4

* Multi-device sync
* Team mode
* Shared goals
* AI executive assistant

---

# Final Product Goal

HabitOS should become the terminal equivalent of:

* Notion
* Todoist
* Streaks
* RescueTime
* Rewind
* Motion

combined into a single local-first AI operating system that helps users consistently achieve long-term goals.
