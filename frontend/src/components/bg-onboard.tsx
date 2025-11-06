interface BgOnboardProps {
  /** Additional CSS classes */
  class?: string;
}

const BgOnboard = (rawProps: BgOnboardProps) => {
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
    <>
      {/* Scattered stars */}
      {stars.map((star, i) => (
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
    </>
  );
};

export default BgOnboard;
