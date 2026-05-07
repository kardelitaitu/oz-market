import re

file_path = "C:/My Script/project-the-marketplace/backend/server/src/repositories/listings.rs"

with open(file_path, "r") as f:
    content = f.read()

# Fix 1: Find all "ListingSummary {" constructions and add seller fields if missing
# Pattern: "ListingSummary {" or "let summary = ListingSummary {"
pattern1 = r'(ListingSummary \{([^}]*)\})'
matches = re.findall(pattern1, content, re.DOTALL)

for match in matches:
    block = match.group(0)
    inner = match.group(1)
    
    # Check if seller fields are missing
    if 'seller_name' not in inner:
        # Add seller fields before the closing brace
        # Find the last "}," or "}," in the block
        last_brace = block.rfind('},')
        if last_brace != -1:
            # Add seller fields
            seller_fields = """
            // Seller fields (read-only, None for now)
            seller_name: None,
            seller_rating: None,
            seller_verified: None,"""
            
            new_block = block[:last_brace] + seller_fields + block[last_brace:]
            content = content.replace(block, new_block)
            print(f"Added seller fields to ListingSummary construction")

# Fix 2: Find "listing: marketplace_api_contract::ListingPayload {" and add marketplace fields
pattern2 = r'(listing: marketplace_api_contract::ListingPayload \{([^}]*)\})'
matches2 = re.findall(pattern2, content, re.DOTALL)

for match in matches2:
    block = match.group(0)
    inner = match.group(1)
    
    # Check if marketplace fields are missing
    if 'sku' not in inner:
        # Add marketplace fields before the closing brace
        last_brace = block.rfind('},')
        if last_brace != -1:
            # Add marketplace fields
            mp_fields = """
            // NEW: Marketplace fields
            sku,
            quantity: if quantity == 1 { None } else { Some(quantity as u32) },
            shipping_info: shipping_info.and_then(|v| serde_json::from_value(v).ok()),
            condition_details,
            seller_notes,"""
            
            new_block = block[:last_brace] + mp_fields + block[last_brace:]
            content = content.replace(block, new_block)
            print(f"Added marketplace fields to ListingPayload construction")

# Write back
with open(file_path, "w") as f:
    f.write(content)

print("Script completed - check for any remaining issues")
