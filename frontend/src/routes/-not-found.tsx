import { useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, For, onMount } from "solid-js";
import { Button } from "~/components";
import { ButtonVariant } from "~/utils/color";

export default function NotFound() {
	const navigate = useNavigate();
	const [stars, setStars] = createSignal<
		{ top: string; left: string; size: number; delay: string; duration: string }[]
	>([]);
	onMount(() => {
		setStars(
			Array.from({ length: 25 }, () => ({
				top: `${Math.random() * 100}%`,
				left: `${Math.random() * 100}%`,
				size: Math.random() * 5,
				delay: `${Math.random() * 3}s`,
				duration: `${Math.random() * 2 + 1.5}s`,
			}))
		);
	});

	const randomizeDuration = (element: HTMLDivElement) => {
		const newDuration = Math.random() * 2 + 1.5; // Random duration between 1.5-3.5s
		element.style.animationDuration = `${newDuration}s`;
	};

	return (
		<>
			<Title>Not Found | Patr</Title>
			<main class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden">
				{/* Starry background */}
				<div class="absolute inset-0 overflow-hidden pointer-events-none" aria-hidden="true">
					<img src="/images/starry-sky.svg" alt="" class="w-full h-full object-cover opacity-60" />
				</div>
				{/* Scattered stars */}
				<For each={stars()}>
					{(star) => (
						<div
							ref={(el) => {
								el.addEventListener("animationiteration", () => randomizeDuration(el));
							}}
							class="absolute bg-white rounded-full animate-pulse"
							aria-hidden="true"
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
					alt=""
					aria-hidden="true"
					class="absolute bottom-0 left-0 pointer-events-none z-0 w-40 md:w-auto opacity-40 md:opacity-100"
				/>
				<img
					src="/images/planet.svg"
					alt=""
					aria-hidden="true"
					class="hidden md:block absolute top-[-10%] right-[-5%] pointer-events-none z-0 w-[15%]"
				/>
				<img
					src="/images/spaceship.svg"
					alt=""
					aria-hidden="true"
					class="
					hidden md:block absolute top-[5%] right-[5%] pointer-events-none
					z-0 w-[15%] animate-[float_25s_ease-in-out_infinite]
					rotate-[-20deg] scale-x-[-1]
					"
				/>
				<img
					src="/images/planet.svg"
					alt=""
					aria-hidden="true"
					class="hidden md:block absolute top-[15%] left-[5%] pointer-events-none z-0 w-[15%]"
				/>
				<img
					src="/images/patr-lowercase.png"
					alt=""
					aria-hidden="true"
					class="absolute top-0 left-0 pointer-events-none z-0 mt-4 ml-3 md:mt-6 md:ml-4 w-24 md:w-[15%]"
				/>

				{/* 404 Content Card */}
				<section class="bg-[#1a0f2e]/70 backdrop-blur-md p-6 md:p-12 rounded-2xl md:rounded-3xl shadow-2xl w-full max-w-125 relative z-10 border border-[#2a1f3d] text-center mx-auto">
					{/* 404 Number */}
					<div class="mb-4 md:mb-6">
						<h1 class="text-[72px] md:text-[120px] font-bold text-transparent bg-clip-text bg-linear-to-r from-primary via-orange-400 to-primary leading-none">
							404
						</h1>
					</div>

					{/* Error Message */}
					<div class="mb-6 md:mb-8">
						<h2 class="text-2xl md:text-3xl font-semibold text-white mb-2 md:mb-3">Lost in Space</h2>
						<p class="text-grey text-sm md:text-base leading-relaxed">
							Oops! Looks like you've drifted into uncharted territory. The page you're looking for
							doesn't exist in our galaxy.
						</p>
					</div>

					{/* Action Buttons */}
					<div class="space-y-3">
						<Button
							variant={ButtonVariant.Contained}
							class="w-full py-3 md:py-3.5 text-sm md:text-[15px] font-semibold rounded-full"
							onClick={() => navigate({ to: "/" })}
						>
							Return Home
						</Button>
						<Button
							onClick={() => window.history.back()}
							class="w-full py-3 md:py-3.5 text-sm md:text-[15px] font-semibold border-2 border-primary text-primary hover:bg-primary hover:text-secondary transition-all duration-200"
						>
							Go Back
						</Button>
					</div>

					{/* Help Text */}
					<div class="mt-6 md:mt-8 pt-4 md:pt-6 border-t border-secondary-medium">
						<p class="text-grey text-xs md:text-sm">
							Need help?{" "}
							<a
								href="https://github.com/patr-cloud/patr/issues"
								target="_blank"
								rel="noopener noreferrer"
								class="text-primary hover:underline"
							>
								Report an Issue
							</a>
						</p>
					</div>
				</section>
			</main>
		</>
	);
}
