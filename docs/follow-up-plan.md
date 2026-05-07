# Follow-up Plan: Complete API Improvements

> **Status**: Blocked on `listings.rs` compilation errors  
> **Time spent**: 2+ hours going in circles with complex edits  
> **Next**: You decide how to proceed

---

## Current State

### ✅ **Completed:**
1. **API Contract** (`backend/crates/api-contract/src/listing.rs`)
   - Added `ShippingInfo` struct
   - Added fields to `ListingPayload`: `sku`, `quantity`, `shipping_info`, `condition_details`, `seller_notes`
   - Added fields to `ListingSummary`: `seller_name`, `seller_rating`, `seller_verified`

2. **Database Model** (`backend/server/src/models/db.rs`)
   - Added new fields to `ListingRow`
   - Updated `into_payload()` to map new fields

3. **OpenAPI Spec** (`docs/specs/openapi.yaml`)
   - Partially updated `ListingPayload` and `ListingSummary` schemas

4. **Migration** (`backend/server/migrations/0004_add_marketplace_fields.sql`)
   - Created migration file

### ❌ **Blocked:**
- **`listings.rs`** (`backend/server/src/repositories/listings.rs`)
   - Compilation errors in `row_to_summary()` and `ListingSummary` constructors
   - Tried multiple approaches: `edit` tool, Python scripts, `bash` — all failed
   - Issue: Complex nested braces, function boundaries, Python heredoc syntax errors

---

## Remaining Work (Exact Steps)

### Step 1: Fix `row_to_summary()` in `listings.rs`

**File**: `backend/server/src/repositories/listings.rs`  
**Function**: `row_to_summary()` (starts around line 220)

**What needs to be done:**
1. Extract new fields from the database row:
   ```rust
   let sku = row.try_get::<Option<String>>("sku").map_err(|e| storage(e.to_string()))?;
   let quantity = row.try_get::<i32>("quantity").map_err(|e| storage(e.to_string()))?;
   let shipping_info = row.try_get::<Option<serde_json::Value>>("shipping_info").map_err(|e| storage(e.to_string()))?;
   let condition_details = row.try_get::<Option<String>>("condition_details").map_err(|e| storage(e.to_string()))?;
   let seller_notes = row.try_get::<Option<String>>("seller_notes").map_err(|e| storage(e.to_string()))?;
   ```

2. Add these fields to the `ListingPayload` construction:
   ```rust
   listing: marketplace_api_contract::ListingPayload {
       // ... existing fields ...
       sku,
       quantity: if quantity == 1 { None } else { Some(quantity as u32) },
       shipping_info: shipping_info.and_then(|v| serde_json::from_value(v).ok()),
       condition_details,
       seller_notes,
   }
   ```

3. Add seller fields to `ListingSummary` construction (can be `None` for now):
   ```rust
   Ok(ListingSummary {
       listing_id,
       status,
       version,
       listing: /* ... */,
       seller_name: None,
       seller_rating: None,
       seller_verified: None,
   })
   ```

### Step 2: Fix `InMemoryListingRepository::insert_listing()`

**File**: `backend/server/src/repositories/listings.rs`  
**Function**: `insert_listing()` in `InMemoryListingRepository` (around line 115)

**What needs to be done:**
- Update the `ListingSummary` construction to include seller fields:
  ```rust
  let summary = ListingSummary {
      listing_id: self.next_id(),
      status: ListingStatus::Active,
      version: 1,
      listing: request.listing.clone(),
      seller_name: None,
      seller_rating: None,
      seller_verified: None,
  };
  ```

### Step 3: Update `insert_listing()` in `PostgresListingRepository`

**File**: `backend/server/src/repositories/listings.rs`  
**Function**: `insert_listing()` in `PostgresListingRepository` (around line 400+)

**What needs to be done:**
1. Update the INSERT query to include new columns:
   ```sql
   INSERT INTO listings (
       listing_id, owner_id, schema_version, category, product_name, "condition",
       price_currency, price_amount, country_code, country_name, city,
       picture_urls, description, attributes, status, version, create_idempotency_key,
       search_text, sku, quantity, shipping_info, condition_details, seller_notes
   ) VALUES (...)
   ```

2. Update the `summary_to_row()` call to include new fields

### Step 4: Verify SELECT Query in `fetch_rows()`

**File**: `backend/server/src/repositories/listings.rs`  
**Function**: `fetch_rows()` (around line 280)

**What needs to be done:**
- Verify the SELECT includes new columns: `sku, quantity, shipping_info, condition_details, seller_notes`
- (I may have already done this in a previous edit)

### Step 5: Update `get_listing()` to Fetch Seller Info (Optional)

**File**: `backend/server/src/repositories/listings.rs`  
**Function**: `get_listing()` in `PostgresListingRepository`

**What needs to be done:**
- Option A: Skip for now (seller fields can be `None`)
- Option B: Add a JOIN with `seller_accounts` table to fetch `display_name`, `trust_level`
- Option C: Make separate query to `seller_accounts` table after fetching listing

---

## Recommended Approach to Fix

### Option A: Manual Edit (You or IDE)
1. Open `backend/server/src/repositories/listings.rs` in an IDE
2. Find `row_to_summary()` function
3. Add the new field extractions and update the constructors
4. Compile and fix any remaining errors

### Option B: Rewrite File (Last Resort)
1. Backup current file: `cp listings.rs listings.rs.backup`
2. Write a clean version with all fixes
3. Compile and test

### Option C: Skip and Commit (Pragmatic)
1. Commit current progress (even with compilation errors)
2. Create a GitHub issue with the exact steps above
3. Let another contributor or future agent fix it

---

## Git Status

```
A  backend/server/migrations/0004_add_marketplace_fields.sql
M  backend/server/src/models/db.rs
M  backend/server/src/repositories/listings.rs
A  docs/api-improvements.md
```

**Commit command**:
```bash
cd "C:/My Script/project-the-marketplace"
git add -A
git commit -m "feat(api): Add marketplace fields (INCOMPLETE - compilation errors)

- Add new fields to API contract, DB model, migration
- Update into_payload() in db.rs
- Partial update to listings.rs (BLOCKED on row_to_summary())
- See docs/follow-up-plan.md for remaining steps"
git push origin main
```

---

## Time Spent

- **2+ hours** going in circles with complex edits
- Multiple failed attempts: `edit` tool, Python scripts, `bash`
- Main issue: Complex nested braces in `row_to_summary()` function

---

**Recommendation**: Let me commit what we have, push, and let YOU decide how to proceed. I've created this detailed plan so you or another agent can continue.

**Would you like me to:**
1. ✅ **Commit and push** current progress (with compilation errors)
2. 📝 **Try ONE more thing** - maybe a simpler approach?
3. 🚀 **Move on** to something else (mobile client, different feature)

Let me know! 🚀
