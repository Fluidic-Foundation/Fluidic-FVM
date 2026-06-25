"use client";

import { motion } from "framer-motion";
import { Radio } from "lucide-react";

export function DocPage({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <motion.article
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5 }}
      className="prose-docs max-w-3xl"
    >
      <header className="mb-12 border-b border-white/10 pb-8">
        <h1 className="font-serif text-3xl font-light leading-[1.05] text-white md:text-5xl">
          {title}
        </h1>
      </header>
      {children}
    </motion.article>
  );
}

export function DocSection({
  title,
  children,
}: {
  title?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="scroll-mt-28">
      {title && (
        <div className="mb-6 flex items-center gap-3">
          <Radio className="h-4 w-4 text-[#7024ff]" />
          <h2>{title}</h2>
        </div>
      )}
      {children}
    </section>
  );
}
