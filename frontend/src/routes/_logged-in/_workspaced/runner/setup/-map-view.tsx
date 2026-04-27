import { onCleanup, onMount } from "solid-js";
import L from "leaflet";
import "leaflet/dist/leaflet.css";

export const MapView = (props: { lat: number; lng: number }) => {
	let mapEl!: HTMLDivElement;
	let map: L.Map | null = null;

	onMount(() => {
		map = L.map(mapEl).setView([props.lat, props.lng], 12);

		// Base map (no labels) — gets pushed toward purple via CSS filter
		L.tileLayer("https://{s}.basemaps.cartocdn.com/dark_nolabels/{z}/{x}/{y}{r}.png", {
			attribution: "© OpenStreetMap · © CARTO",
			subdomains: "abcd",
			maxZoom: 19,
			className: "leaflet-base-purple",
		}).addTo(map);

		// Labels-only overlay — gets pushed toward yellow independently
		L.tileLayer("https://{s}.basemaps.cartocdn.com/dark_only_labels/{z}/{x}/{y}{r}.png", {
			attribution: "",
			subdomains: "abcd",
			maxZoom: 19,
			className: "leaflet-labels-yellow",
			pane: "shadowPane",
		}).addTo(map);

		// IP geolocation is only city-accurate; show the area, not a fake-precise pin.
		L.circle([props.lat, props.lng], {
			radius: 4000,
			color: "#f89b41",
			weight: 2,
			fillColor: "#f89b41",
			fillOpacity: 0.18,
		}).addTo(map);

		// Layout often hasn't settled when onMount runs; force Leaflet to
		// re-measure once the browser has painted.
		requestAnimationFrame(() => map?.invalidateSize());
	});

	onCleanup(() => {
		map?.remove();
		map = null;
	});

	return <div ref={mapEl!} class="leaflet-dim h-72 w-full border border-border-color rounded-xs" />;
};
