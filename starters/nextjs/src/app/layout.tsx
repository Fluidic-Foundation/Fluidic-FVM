export const metadata = {
  title: "My Fluidic App",
  description: "A starter app built on the Fluidic testnet",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
