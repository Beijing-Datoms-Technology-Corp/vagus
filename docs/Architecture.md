# Vagus Architecture

## System Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   LLM Planner   │───▶│   Vagus Layer   │───▶│   Executor      │
│                 │    │                 │    │   (Robot/AI)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌─────────────────┐
                       │   Blockchain    │
                       │   (Safety       │
                       │    State)       │
                       └─────────────────┘
```

## Component Interactions

1. **Planner** generates EIP-712 signed intents
2. **Vagal Brake** applies ANS-based scaling
3. **Capability Issuer** mints time-bound tokens
4. **Gateway** monitors execution and submits telemetry
5. **Oracle** computes VTI and updates ANS state
6. **Reflex Arc** triggers emergency revocation when needed
