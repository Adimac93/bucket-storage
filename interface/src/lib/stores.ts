import { writable } from "svelte/store";

export interface AuthKey {
  keyId: string;
  key: string;
}

const itemName = "auth-key";

function parseKey() {
  const storage = localStorage.getItem(itemName);
  if (storage != null) {
    try {
      const parsed: AuthKey = JSON.parse(storage);
      return parsed;
    } catch (e) {
      console.error(e);
    }
  }
  return null;
}

export const key = writable(parseKey());
key.subscribe((k) => {
  localStorage.setItem(itemName, JSON.stringify(k));
});
