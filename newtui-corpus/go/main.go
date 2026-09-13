package main

import (
	"fmt"
	"io"
	"os"
)

func run(args []string, out io.Writer) error {
	if len(args) != 1 {
		return fmt.Errorf("usage: go-consumer CORPUS.json")
	}
	f, err := os.Open(args[0])
	if err != nil {
		return err
	}
	defer f.Close()
	raw, err := io.ReadAll(io.LimitReader(f, artifactLimit+1))
	if err != nil {
		return err
	}
	a, err := ReadArtifact(raw)
	if err != nil {
		return err
	}
	n, err := Check(a, NewDial)
	if err != nil {
		return err
	}
	_, err = fmt.Fprintf(out, "conformant: %d observations (independent Go component)\n", n)
	return err
}
func main() {
	if err := run(os.Args[1:], os.Stdout); err != nil {
		fmt.Fprintln(os.Stderr, "corpus:", err)
		os.Exit(1)
	}
}
