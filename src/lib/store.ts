const STORAGE_KEY = "woodeye_last_repo";

export function getLastRepoPath(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

export function saveLastRepoPath(path: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, path);
  } catch {
    // Ignore storage errors
  }
}

export function clearLastRepoPath(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Ignore storage errors
  }
}
