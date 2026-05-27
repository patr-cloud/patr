export function randomIPv4(): string {
  const octet = () => Math.floor(Math.random() * 254) + 1;
  return `${octet()}.${octet()}.${octet()}.${octet()}`;
}
