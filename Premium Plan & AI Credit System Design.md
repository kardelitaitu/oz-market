Premium Plan & AI Credit System Design
**Marketplace with AI Sales Agents**  
**Version 1.0**  
**Date:** May 2026

## 1. Overview

We offer a **Premium subscription** that gives users powerful AI Agents for listing creation, sales, and negotiation, combined with higher trust levels and increased platform limits.

The plan is designed to be **generous yet profitable** through smart credit expiration and usage patterns.

## 2. Pricing Tiers

| Plan          | Monthly Price | Yearly Price (per month) | Yearly Price (total) | Best For |
|---------------|---------------|--------------------------|----------------------|----------|
| **Premium**   | **$5**        | **$4.17**                | **$50**              | Most users |
| **Pro** (Future) | $12         | $9.17                    | $110                 | Heavy sellers |

**Early Launch Special**: First 3 months at $5/month, then standard pricing.

## 3. What Premium Users Get ($5/month)

### AI Credits
- **$6 worth** of DeepInfra API credits per month
- Credits are added on the 1st of every month (or on subscription renewal date)
- **Expiration**: 60 days from the date they are credited
- Unused credits **cannot accumulate indefinitely** (encourages consistent usage)

### Trust & Marketplace Benefits
- Automatic **Trust Level 2** (Trusted)
- Higher listing limits (e.g., 50–100 listings/day vs 5–10 for free users)
- AI Agent usage with priority routing
- Ability to receive **Premium vouches** (stronger trust signal)
- Better visibility and ranking for listings
- Faster support response

### Contact Exchange
- Can exchange contact information when `trust_level >= 2`
- Higher daily contact exchange limit

## 4. Credit System Rules

- Credits are measured in **USD value** (not raw tokens)
- Users can see real-time credit balance and estimated conversation count
- System automatically uses the most cost-efficient model first, unless user selects a stronger model
- **Rollover Policy**: Maximum **50%** of unused credits can roll over to the next month (max $3)
- Any credits older than 60 days automatically expire

### Credit Usage Examples (Approximate)

| Model Tier              | $6 Credit Value ≈          | Estimated Negotiation Turns |
|-------------------------|----------------------------|-----------------------------|
| Light / Fast models     | 25 – 40 million tokens     | 3,000 – 5,000 turns |
| Balanced (Recommended)  | 18 – 25 million tokens     | 2,000 – 3,500 turns |
| Strong Reasoning        | 10 – 14 million tokens     | 1,200 – 2,000 turns |

## 5. Extra Credit Top-ups

Users can buy additional credits anytime:

- $10 → $11 worth of credits
- $25 → $28 worth of credits
- $50 → $58 worth of credits

**Markup**: ~10–15% (this becomes a major profit center)

## 6. Unit Economics (Internal View)

**Per Monthly Premium User:**

- Revenue: **$5.00**
- API Credit Cost: **~$6.00** (DeepInfra)
- Payment Processing: **~$0.40**
- **Gross Margin before breakage**: -~$1.40

**With realistic breakage & behavior**:
- Average usage: **$4.00 – $4.50** out of $6
- Expected Gross Profit: **+$0.10 to +$0.80** per user
- Extra credit purchases from heavy users significantly increase profitability

**Key Profit Levers**:
- Credit breakage (unused & expired)
- Extra credit top-ups
- Yearly subscriptions (higher retention)
- Transaction fees (future)

## 7. Technical Implementation Notes

- Use **LiteLLM** as proxy layer for easy model routing and usage tracking
- Primary provider: **DeepInfra**
- Store user credits in database with `credited_at` and `expires_at`
- Background job to expire credits daily
- Dashboard for users showing credit usage, history, and estimated conversations left

## 8. Future Enhancements

- Pro tier with higher credits + access to premium models
- Usage-based tiers
- AI Agent performance bonuses (more credits for high-rated agents)
- Referral credits

---

**End of Document**

Approved for implementation.