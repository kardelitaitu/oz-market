Trust & Verification System Design
**Marketplace with AI Sales Agents**  
**Version 2.0 (Recommended)**  
**Date:** May 2026

## 1. Overview

This is a general-purpose marketplace that allows high-volume listings (spam-tolerant) while maintaining quality, trust, and safety through a multi-signal verification system.

AI Agents act as autonomous salespersons and negotiators. Real contact information (phone/WhatsApp) is only exchanged when negotiations reach near the asking price **and** both parties have sufficient trust.

## 2. Core Objectives

- Strong protection against scams during contact exchange
- Low friction for new users
- Robust monetization through Premium ($1/month)
- Fair and fast trust building (especially in early stages)
- High performance at scale (50k+ operations/second)
- Balanced system that rewards both Premium users and good behavior

## 3. Trust Levels

| Level | Name              | Minimum Requirements                              | Key Privileges |
|-------|-------------------|---------------------------------------------------|----------------|
| **0** | Unverified        | No phone verification                             | Browse only |
| **1** | Basic             | Phone verified                                    | Create listings (low limits) |
| **2** | Trusted           | Phone + Moderate trust signals                    | Full AI Agent usage, contact exchange |
| **3** | Highly Trusted    | Phone + Strong signals (permanent vouch, high activity, good ratings) | Boosted visibility, highest limits, special badges |

## 4. Trust Signals

| Signal                        | Weight | Notes |
|------------------------------|--------|-------|
| Phone Verified               | 30     | Foundational requirement |
| Active Vouches               | 12 per vouch | From premium users |
| Permanent Vouches            | 25 per vouch | From yearly premium users |
| Successful Transactions      | 8 per transaction | Max 30 points |
| Rating Score                 | 5 × average rating | Based on post-contact reviews |
| Account Age                  | 1 per 3 days | Max 10 points |

**Final `trust_score`** (0–100) is calculated from the above and mapped to `trust_level`.

## 5. Premium & Vouching Rules

- **Price**: $1 per month
- **Yearly Premium**: Strongly recommended (12 months billed upfront)

### Vouching Rules
- Only **Premium** users can vouch for others.
- One user can vouch for another user **only once**.
- **Monthly Premium** → Temporary vouch (with 14-day grace period after expiration).
- **Yearly Premium** → **Permanent** vouch (never expires).
- Yearly vouches carry **higher weight** (+25 points vs +15 for monthly).

## 6. Database Schema

### Main Table: `users`

```c
users (
  id uuid PRIMARY KEY,
  
  -- Premium & Billing
  is_premium boolean DEFAULT false,
  premium_plan text,                    -- 'monthly' | 'yearly'
  premium_expires_at timestamptz,
  premium_subscription_id text,
  
  -- Phone Verification (Critical)
  phone_number text UNIQUE,
  phone_verified boolean DEFAULT false,
  phone_verified_at timestamptz,
  
  -- Trust Signals
  active_vouches_count int DEFAULT 0,
  permanent_vouches_count int DEFAULT 0,
  successful_transactions int DEFAULT 0,
  total_ratings int DEFAULT 0,
  positive_rating_score float DEFAULT 0.0,   -- 0.0 - 5.0
  
  -- Computed Fields
  trust_score int DEFAULT 0,                -- 0-100
  trust_level smallint DEFAULT 0,           -- 0-3
  last_trust_update timestamptz,
  
  ai_agent_enabled boolean DEFAULT true,
  status text DEFAULT 'active'
);
```



```mermaid
flowchart TD
    subgraph Signals ["Trust Signals"]
        Phone[Phone Verified\n+30]
        Vouches[Premium Vouches\nActive + Permanent]
        Activity[Activity & Ratings\nTransactions + Reviews]
        Age[Account Age\n+10 max]
    end

    subgraph Engine ["Trust Engine"]
        Score[Trust Score 0-100]
        Level[Trust Level 0-3]
    end

    subgraph Benefits ["Benefits by Level"]
        L0[Browse Only]
        L1[Basic Listing]
        L2[Full AI Agent + Contact Exchange]
        L3[Boosted Visibility + Max Limits]
    end

    Signals --> Score
    Score --> Level
    Level --> Benefits

    style Level fill:#4ade80,stroke:#166534
    style L3 fill:#22c55e,stroke:#166534
```


```mermaid

flowchart TD
    A[Premium User A] -->|Wants to Vouch| B{Is Premium Active?}
    
    B -->|No| Reject[Cannot Vouch]
    B -->|Yes| C{Plan Type?}
    
    C -->|Monthly| D[Temporary Vouch\n+15 points]
    C -->|Yearly| E[Permanent Vouch\n+25 points]
    
    D --> F[Add to user_vouches]
    E --> F
    
    F --> G[Update vouched user's\ntrust_score + trust_level]
    
    H[Monthly Premium Expires] --> I[14-day Grace Period]
    I --> J{Still Expired?}
    J -->|Yes| K[Revoke Temporary Vouch]
    J -->|No| L[Vouch Remains Active]
    
    K --> M[Recalculate trust_score]
    
```


```mermaid
flowchart TD
    Start[AI Agent Requests Contact Exchange] --> A{Both Phone Verified?}
    
    A -->|No| Reject1[❌ Reject\nRequire Phone Verification]
    A -->|Yes| B{Both Trust Level >= 2?}
    
    B -->|No| Reject2[❌ Insufficient Trust]
    B -->|Yes| C{High Value Item?}
    
    C -->|No| Approve[✅ Approve Exchange]
    C -->|Yes| D[Require Trust Level 3\nor Human Seller Approval]
    
    D -->|Approved| Approve
    D -->|Not Approved| Reject3[❌ Declined]
    
    style Approve fill:#4ade80,stroke:#166534
    style Reject1 fill:#f87171,stroke:#991b1b
    style Reject2 fill:#f87171,stroke:#991b1b
```


```mermaid

flowchart TD
    Start[Calculate Trust Score] --> Phone{Phone Verified?}
    
    Phone -->|No| Score0[Trust Score = 0\nLevel 0]
    Phone -->|Yes| Base[Base Score = 30]
    
    Base --> Vouches[Vouches\nActive ×12 + Permanent ×25]
    Vouches --> Activity[Transactions + Ratings]
    Activity --> Age[Account Age\nMax +10]
    
    Age --> Final[Final Trust Score 0-100]
    Final --> Level{Map to Level}
    
    Level --> L1[30-54 → Level 1]
    Level --> L2[55-79 → Level 2]
    Level --> L3[80+ → Level 3]
    
    style Final fill:#a5b4fc,stroke:#4338ca
```



```mermaid
erDiagram
    USERS ||--o{ USER_VOUCHES : "vouches"
    USERS ||--o{ USER_VOUCHES : "receives"

    USERS {
        uuid id PK
        boolean is_premium
        string premium_plan
        timestamp premium_expires_at
        boolean phone_verified
        int active_vouches_count
        int permanent_vouches_count
        int trust_score
        smallint trust_level
    }

    USER_VOUCHES {
        uuid id PK
        uuid voucher_id FK "References USERS.id"
        uuid vouched_user_id FK "References USERS.id"
        string vouch_type "temporary/permanent"
        int points_given
        timestamp created_at
        timestamp revoked_at
    }
```



