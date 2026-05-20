export type Category =
  | 'laptop' | 'phone' | 'tablet' | 'desktop' | 'monitor'
  | 'accessory' | 'camera' | 'audio' | 'gaming'
  | 'appliance' | 'furniture' | 'vehicle_part' | 'other';

export type Condition = 'new' | 'used' | 'refurbished';

export type ListingStatus = 'draft' | 'active' | 'reserved' | 'sold' | 'archived';

export type ListingType = 'product' | 'service' | 'property';

export type NegotiationStatus =
  | 'open' | 'countered' | 'near_close' | 'reserved'
  | 'contact_requested' | 'contact_revealed' | 'closed' | 'cancelled';

export interface Price {
  currency: string;
  amount: number;
}

export interface ListingLocation {
  country_code: string;
  country_name: string;
  city: string;
  latitude?: number;
  longitude?: number;
  geolocation_opt_out?: boolean;
}

export interface ListingPayload {
  schema_version: string;
  owner_id: string;
  listing_type: ListingType;
  category?: Category;
  title: string;
  condition?: Condition;
  price: Price;
  location: ListingLocation;
  picture_urls: string[];
  description: string;
  attributes?: unknown;
  sku?: string;
  quantity?: number;
  shipping_info?: unknown;
  condition_details?: string;
  seller_notes?: string;
  service_type?: 'local' | 'online';
  hourly_rate?: number;
  project_rate?: number;
  qualifications?: string[];
  service_radius_km?: number;
  property_transaction_type?: 'rent' | 'sale';
  property_sub_type?: 'building' | 'house' | 'apartment' | 'land';
  area_sqm?: number;
  bedrooms?: number;
  bathrooms?: number;
  year_built?: number;
  lot_size_sqm?: number;
  zoning?: string;
}
