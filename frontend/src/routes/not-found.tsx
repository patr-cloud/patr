import { A } from "@solidjs/router";
import { For } from "solid-js";
import Button from "~/components/button";
import { ButtonVariant } from "~/utils/color";

export default function NotFound() {
	// Generate random stars
	const stars = Array.from({ length: 25 }, () => ({
		top: `${Math.random() * 100}%`,
		left: `${Math.random() * 100}%`,
		size: Math.random() * 5,
		delay: `${Math.random() * 3}s`,
		duration: `${Math.random() * 2 + 1.5}s`,
	}));

	const randomizeDuration = (element: HTMLDivElement) => {
		const newDuration = Math.random() * 2 + 1.5; // Random duration between 1.5-3.5s
		element.style.animationDuration = `${newDuration}s`;
	};

	return (
		<main class="min-h-screen w-full bg-[#0d0526] flex items-center justify-center p-4 relative overflow-hidden">
			{/* Starry background */}
			<div class="absolute inset-0 overflow-hidden pointer-events-none">
				<img src="/images/starry-sky.svg" alt="Starry Sky" class="w-full h-full object-cover opacity-60" />
			</div>
			{/* Scattered stars */}
			<For each={stars}>
				{(star) => (
					<div
						ref={(el) => {
							el.addEventListener("animationiteration", () => randomizeDuration(el));
						}}
						class="absolute bg-white rounded-full animate-pulse"
						style={{
							top: star.top,
							left: star.left,
							width: `${star.size}px`,
							height: `${star.size}px`,
							"animation-delay": star.delay,
							"animation-duration": star.duration,
						}}
					/>
				)}
			</For>

			<img
				src="/images/astronaut.svg"
				alt="Floating Astronaut"
				class="absolute bottom-0 left-0 pointer-events-none z-0"
			/>
			<img
				src="/images/planet.svg"
				alt="Purple Planet"
				class="absolute top-[-10%] right-[-5%] pointer-events-none z-0 w-[15%]"
			/>
			<img
				src="/images/spaceship.svg"
				alt="Spaceship"
				class="
          absolute top-[5%] right-[5%] pointer-events-none 
          z-0 w-[15%] animate-[float_25s_ease-in-out_infinite]
          rotate-[-20deg] scale-x-[-1]
        "
			/>
			<img
				src="/images/planet.svg"
				alt="Purple Planet"
				class="absolute top-[15%] left-[5%] pointer-events-none z-0 w-[15%]"
			/>
			<img
				src="/images/patr-lowercase.png"
				alt="Patr Logo"
				class="absolute top-0 left-0 pointer-events-none z-0 mt-6 ml-4 w-[15%]"
			/>

			{/* 404 Content Card */}
			<section class="bg-[#1a0f2e]/70 backdrop-blur-md p-12 rounded-3xl shadow-2xl w-full max-w-125 relative z-10 border border-[#2a1f3d] text-center">
				{/* 404 Number */}
				<div class="mb-6">
					<h1 class="text-[120px] font-bold text-transparent bg-clip-text bg-linear-to-r from-primary via-orange-400 to-primary leading-none animate-pulse">
						404
					</h1>
				</div>

				{/* Error Message */}
				<div class="mb-8">
					<h2 class="text-3xl font-semibold text-white mb-3">Lost in Space</h2>
					<p class="text-gray-400 text-base leading-relaxed">
						Oops! Looks like you've drifted into uncharted territory. The page you're looking for doesn't exist in our
						galaxy.
					</p>
				</div>

				{/* Action Buttons */}
				<div class="space-y-3">
					<A href="/" class="block">
						<Button variant={ButtonVariant.Contained} class="w-full py-3.5 text-[15px] font-semibold rounded-full">
							Return Home
						</Button>
					</A>
					<Button
						onClick={() => window.history.back()}
						class="w-full py-3.5 text-[15px] font-semibold border-2 border-primary text-primary hover:bg-primary hover:text-secondary transition-all duration-200"
					>
						Go Back
					</Button>
				</div>

				{/* Help Text */}
				<div class="mt-8 pt-6 border-t border-secondary-medium">
					<p class="text-gray-500 text-sm">
						Need help?{" "}
						<A href="/contact" class="text-primary hover:underline">
							Contact Support
						</A>
					</p>
				</div>
			</section>

			{/* Fun floating animation */}
			<style>{`
				@keyframes float {
					0%, 100% {
						transform: translateY(0px) rotate(0deg);
					}
					50% {
						transform: translateY(-20px) rotate(5deg);
					}
				}
				.animate-float {
					animation: float 6s ease-in-out infinite;
				}
			`}</style>
		</main>
	);
}
