/**
 * Token Storage Abstraction Interface.
 *
 * Encapsulates authentication token persistence logic so the application is not
 * directly tightly coupled to localStorage.
 *
 * PROTOTYPE NOTE: For this demonstration prototype, tokens are stored in localStorage.
 * In a production healthcare deployment, token storage should be evaluated for
 * HttpOnly, Secure, SameSite=Strict cookies or a Web Worker token manager to
 * protect against XSS token exfiltration.
 */

export interface TokenStorage {
  getToken(): string | null
  setToken(token: string): void
  clearToken(): void
}

const STORAGE_KEY = 'dhp_auth_token'

export const localStorageTokenStorage: TokenStorage = {
  getToken(): string | null {
    try {
      return localStorage.getItem(STORAGE_KEY)
    } catch {
      return null
    }
  },
  setToken(token: string): void {
    try {
      localStorage.setItem(STORAGE_KEY, token)
    } catch (e) {
      console.warn('Failed to save auth token to localStorage:', e)
    }
  },
  clearToken(): void {
    try {
      localStorage.removeItem(STORAGE_KEY)
    } catch (e) {
      console.warn('Failed to clear auth token from localStorage:', e)
    }
  },
}

// Active storage implementation used across the application
export const tokenStorage: TokenStorage = localStorageTokenStorage
