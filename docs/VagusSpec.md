# Vagus Protocol Specification

## Overview

Vagus implements a safety layer inspired by the autonomic nervous system's vagal nerve, providing dynamic safety controls for autonomous agents.

## Core Components

### 1. Afferent Evidence Processing (AEP)
- Device-side telemetry collection
- State root commitments
- Evidence verification and archival

### 2. Autonomic Nervous System (ANS) States
- **SAFE**: Normal operation
- **DANGER**: Reduced capability with scaling
- **SHUTDOWN**: Emergency stop

### 3. Vagal Brake
- Dynamic parameter scaling based on ANS state
- Configurable brakeable fields per action type

### 4. Reflex Arc
- Automated capability revocation based on evidence
- Configurable thresholds and rate limiting

### 5. Capability Tokens
- ERC721-like tokens with time-based expiration
- Action-specific permissions with scaling limits
