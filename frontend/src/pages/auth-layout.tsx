import { JSX } from "solid-js";

interface AuthLayoutProps {
  children: JSX.Element;
}

const AuthLayout = (props: AuthLayoutProps) => {
  // Generate random stars for the space theme
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
    <div
      class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden"
      style={{
        "background-image": "url('/images/starry-sky.svg')",
        "background-size": "cover",
        "background-position": "center",
      }}
    >
      {/* Scattered stars */}
      {stars.map((star) => (
        <div
          ref={(el) => {
            el.addEventListener("animationiteration", () =>
              randomizeDuration(el)
            );
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
      ))}

      {/* Space theme decorative elements */}
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
        src="/images/patr.svg"
        alt="Patr Logo"
        class="absolute top-0 left-0 pointer-events-none z-0 mt-6 ml-4 w-[15%]"
      />

      {/* Main content area */}
      <div class="relative z-10 w-full max-w-[32rem]">
        {props.children}
      </div>

      {/* Footer with copyright */}
      <div class="absolute bottom-6 left-0 right-0 text-center">
        <p class="text-gray-500 text-xs">© 2025 Patr. All rights reserved.</p>
      </div>
    </div>
  );
};

export default AuthLayout;