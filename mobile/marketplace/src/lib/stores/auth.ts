import { getClaims } from '$lib/api/commands';

let _loggedIn = false;

export function isLoggedIn(): boolean {
  return _loggedIn;
}

export async function checkAuth(): Promise<boolean> {
  try {
    await getClaims();
    _loggedIn = true;
    return true;
  } catch {
    _loggedIn = false;
    return false;
  }
}

export function setLoggedIn(value: boolean): void {
  _loggedIn = value;
}
