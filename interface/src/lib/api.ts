import { key } from "./stores";

export async function fetchAuthorized(input: RequestInfo | URL, init?: RequestInit) {
    let authKey: string;
    key.subscribe(k => authKey = k)();
    return await fetch(input, {
        headers: {
            Authorization: `Basic ${authKey}`
        },
        ...init
    })
}