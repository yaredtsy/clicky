import type { ReactNode } from "react";
import { Shield } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface PermissionGateProps {
  status: "loading" | "allowed" | "denied" | "error";
  error: string | null;
  onRequest: () => void;
  onRefresh: () => void;
  children: ReactNode;
}

export function PermissionGate({
  status,
  error,
  onRequest,
  onRefresh,
  children,
}: PermissionGateProps) {
  if (status === "allowed") {
    return <>{children}</>;
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-6">
      <Card className="w-full max-w-md text-center">
        <CardHeader>
          <div className="mx-auto mb-2 flex size-12 items-center justify-center rounded-full bg-muted">
            <Shield className="size-6 text-muted-foreground" />
          </div>
          <CardTitle>Accessibility permission required</CardTitle>
          <CardDescription className="text-pretty">
            Claw Inspector needs Accessibility access to read other apps&apos; UI
            structure.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4 text-left text-sm text-muted-foreground">
          {status === "loading" ? (
            <p>Checking permission status…</p>
          ) : null}

          {status === "denied" ? (
            <>
              <ol className="list-decimal space-y-1 pl-5">
                <li>
                  Click <strong className="text-foreground">Request permission</strong>{" "}
                  below
                </li>
                <li>
                  In System Settings, enable{" "}
                  <strong className="text-foreground">claw-kernel</strong>
                </li>
                <li>
                  Click{" "}
                  <strong className="text-foreground">Refresh status</strong> to
                  continue
                </li>
              </ol>
            </>
          ) : null}

          {status === "error" ? (
            <p className={cn("rounded-md border border-destructive/30 bg-destructive/10 p-3 text-destructive")}>
              {error}
            </p>
          ) : null}
        </CardContent>
        <CardFooter className="flex flex-wrap justify-center gap-2">
          {status === "denied" ? (
            <>
              <Button onClick={onRequest}>Request permission</Button>
              <Button variant="outline" onClick={onRefresh}>
                Refresh status
              </Button>
            </>
          ) : null}
          {status === "error" ? (
            <Button variant="outline" onClick={onRefresh}>
              Retry
            </Button>
          ) : null}
          {status === "loading" ? (
            <Button variant="outline" disabled>
              Please wait…
            </Button>
          ) : null}
        </CardFooter>
      </Card>
    </div>
  );
}
