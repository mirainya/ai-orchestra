import { useCallback, useRef, useState } from "react";

interface UseResizableOptions {
  direction: "vertical" | "horizontal";
  initialSize: number;
  minSize?: number;
  maxSize?: number;
}

export function useResizable({ direction, initialSize, minSize = 100, maxSize = 600 }: UseResizableOptions) {
  const [size, setSize] = useState(initialSize);
  const dragging = useRef(false);
  const startPos = useRef(0);
  const startSize = useRef(0);

  const onMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    dragging.current = true;
    startPos.current = direction === "vertical" ? e.clientY : e.clientX;
    startSize.current = size;

    const onMouseMove = (ev: MouseEvent) => {
      if (!dragging.current) return;
      const delta = direction === "vertical"
        ? startPos.current - ev.clientY
        : ev.clientX - startPos.current;
      const next = Math.min(maxSize, Math.max(minSize, startSize.current + delta));
      setSize(next);
    };

    const onMouseUp = () => {
      dragging.current = false;
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    document.body.style.cursor = direction === "vertical" ? "row-resize" : "col-resize";
    document.body.style.userSelect = "none";
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }, [direction, size, minSize, maxSize]);

  const handleProps = {
    onMouseDown,
    style: {
      cursor: direction === "vertical" ? "row-resize" : "col-resize",
      position: "relative" as const,
      zIndex: 5,
    },
  };

  return { size, handleProps };
}
