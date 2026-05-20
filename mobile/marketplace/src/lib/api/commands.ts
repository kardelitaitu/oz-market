import { invoke } from '@tauri-apps/api/core';
import { withRetry } from '$lib/utils/retry';

export interface LoginParams {
  sub: string;
  sellerAccountId?: string;
  buyerAgentId?: string;
  roles: string[];
  scopes: string[];
}

export interface ListingSummary {
  listing_id: string;
  status: string;
  version: number;
  listing: unknown;
  seller_name?: string;
  seller_rating?: number;
  seller_verified?: boolean;
}

export interface SearchResponse {
  items: ListingSummary[];
  applied_sort_by: string;
  next_cursor?: string;
}

export async function health(): Promise<unknown> {
  return invoke('health');
}

export async function login(params: LoginParams): Promise<string> {
  return invoke('login', {
    sub: params.sub,
    sellerAccountId: params.sellerAccountId ?? null,
    buyerAgentId: params.buyerAgentId ?? null,
    roles: params.roles,
    scopes: params.scopes,
  });
}

export async function logout(): Promise<void> {
  return invoke('logout');
}

export async function getClaims(): Promise<string> {
  return invoke('get_claims');
}

export async function getListing(listingId: string): Promise<ListingSummary> {
  return invoke('get_listing', { listingId });
}

export async function searchListings(params: {
  query?: string;
  category?: string;
  limit?: number;
  cursor?: string;
}): Promise<SearchResponse> {
  return invoke('search_listings', {
    query: params.query ?? null,
    category: params.category ?? null,
    limit: params.limit ?? null,
    cursor: params.cursor ?? null,
  });
}

export async function setBaseUrl(url: string): Promise<void> {
  return invoke('set_base_url', { url });
}

export async function getBaseUrl(): Promise<string> {
  return invoke('get_base_url');
}

export interface CreateListingParams {
  title: string;
  description: string;
  listingType: string;
  currency: string;
  amount: number;
  countryCode: string;
  city: string;
  idempotencyKey: string;
}

export async function createListing(params: CreateListingParams): Promise<ListingSummary> {
  return invoke('create_listing', { params });
}

export async function myListings(params: {
  limit?: number;
  cursor?: string;
}): Promise<SearchResponse> {
  return invoke('my_listings', {
    limit: params.limit ?? null,
    cursor: params.cursor ?? null,
  });
}

export interface NegotiationResponse {
  negotiation_id: string;
  listing_id: string;
  buyer_agent_id: string;
  status: string;
  offer_currency: string;
  latest_offer_amount: number;
  offer_history: NegotiationHistoryEntry[];
  reveal_id?: string;
  version: number;
  updated_at: string;
}

export interface NegotiationHistoryEntry {
  entry_id: string;
  entry_type: string;
  offer_currency: string;
  offer_amount: number;
  actor_subject: string;
  actor_role: string;
  idempotency_key: string;
  resulting_status: string;
  created_at: string;
}

export interface ContactRevealResponse {
  reveal_id: string;
  negotiation_id: string;
  reveal_status: string;
  revealed_phone_reference?: string;
  expires_at?: string;
  approved_at?: string;
  updated_at: string;
}

export async function openNegotiation(params: {
  listingId: string;
  currency: string;
  amount: number;
  idempotencyKey: string;
}): Promise<NegotiationResponse> {
  return invoke('open_negotiation', {
    params: {
      listing_id: params.listingId,
      currency: params.currency,
      amount: params.amount,
      idempotency_key: params.idempotencyKey,
    },
  });
}

export async function getNegotiation(negotiationId: string): Promise<NegotiationResponse> {
  return invoke('get_negotiation', {
    params: { negotiation_id: negotiationId },
  });
}

export async function submitOffer(params: {
  negotiationId: string;
  currency: string;
  amount: number;
  idempotencyKey: string;
}): Promise<NegotiationResponse> {
  return invoke('submit_offer', {
    params: {
      negotiation_id: params.negotiationId,
      currency: params.currency,
      amount: params.amount,
      idempotency_key: params.idempotencyKey,
    },
  });
}

export async function acceptNegotiation(
  negotiationId: string,
  idempotencyKey: string,
): Promise<NegotiationResponse> {
  return invoke('accept_negotiation', {
    negotiationId,
    params: { idempotency_key: idempotencyKey },
  });
}

export async function rejectNegotiation(
  negotiationId: string,
  idempotencyKey: string,
): Promise<NegotiationResponse> {
  return invoke('reject_negotiation', {
    negotiationId,
    params: { idempotency_key: idempotencyKey },
  });
}

export async function requestContactReveal(
  negotiationId: string,
  idempotencyKey: string,
): Promise<ContactRevealResponse> {
  return invoke('request_contact_reveal', {
    negotiationId,
    params: { idempotency_key: idempotencyKey },
  });
}

export async function approveContactReveal(
  revealId: string,
  idempotencyKey: string,
): Promise<ContactRevealResponse> {
  return invoke('approve_contact_reveal', {
    revealId,
    params: { idempotency_key: idempotencyKey },
  });
}

// -- Rate Limits ---

export interface RateLimitInfo {
  action: string;
  remaining: number;
  limit: number;
  reset_after_secs: number;
}

export async function getRateLimits(): Promise<RateLimitInfo[]> {
  return invoke('get_rate_limits');
}

// -- Agent ---

export interface AgentAction {
  action_type: string;
  label: string;
  params: unknown;
}

export interface AgentQueryResponse {
  message: string;
  actions: AgentAction[];
  conversation_id: string;
  listing_ids?: string[];
}

export async function agentQuery(params: {
  query: string;
  conversationId?: string;
}): Promise<AgentQueryResponse> {
  return invoke('agent_query', {
    params: {
      query: params.query,
      conversation_id: params.conversationId ?? null,
    },
  });
}

// -- Notifications ---

import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';

export async function requestNotificationPermission(): Promise<boolean> {
  if (await isPermissionGranted()) return true;
  const permission = await requestPermission();
  return permission === 'granted';
}

export function sendLocalNotification(title: string, body: string): void {
  sendNotification({ title, body });
}
