package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"unicode/utf8"

	strictjson "github.com/go-json-experiment/json"
	"github.com/go-json-experiment/json/jsontext"
	cid "github.com/ipfs/go-cid"
	"github.com/ipld/go-ipld-prime/codec/dagcbor"
	"github.com/ipld/go-ipld-prime/codec/dagjson"
	"github.com/ipld/go-ipld-prime/datamodel"
	"github.com/ipld/go-ipld-prime/node/basicnode"
	mh "github.com/multiformats/go-multihash"
	_ "github.com/multiformats/go-multihash/register/blake3"
)

func jsonNode(raw []byte) (datamodel.Node, error) {
	// encoding/json and refmt replace lone UTF-16 surrogates. Check RFC 8259
	// scalar validity before either decoder can normalize malformed input.
	if !jsontext.Value(raw).IsValid() {
		return nil, fmt.Errorf("JSON has invalid Unicode, duplicate keys or syntax")
	}
	b := basicnode.Prototype.Any.NewBuilder()
	// Seed/intent maps are ordinary JSON: '/' does not acquire link semantics.
	if err := (dagjson.DecodeOptions{ParseLinks: false, ParseBytes: false, MaxDepth: 80}).Decode(b, bytes.NewReader(raw)); err != nil {
		return nil, err
	}
	n := b.Build()
	if err := portable(n, 0, 80); err != nil {
		return nil, err
	}
	return n, nil
}

func portable(n datamodel.Node, depth, limit int) error {
	if depth > limit {
		return fmt.Errorf("portable value nesting limit exceeded")
	}
	switch n.Kind() {
	case datamodel.Kind_Null, datamodel.Kind_Bool, datamodel.Kind_Int:
		return nil
	case datamodel.Kind_String:
		s, err := n.AsString()
		if err != nil {
			return err
		}
		if !utf8.ValidString(s) {
			return fmt.Errorf("invalid Unicode string")
		}
		return nil
	case datamodel.Kind_List:
		it := n.ListIterator()
		for !it.Done() {
			_, child, err := it.Next()
			if err != nil {
				return err
			}
			if err := portable(child, depth+1, limit); err != nil {
				return err
			}
		}
	case datamodel.Kind_Map:
		it := n.MapIterator()
		for !it.Done() {
			key, child, err := it.Next()
			if err != nil {
				return err
			}
			if err := portable(key, depth, limit); err != nil {
				return err
			}
			if err := portable(child, depth+1, limit); err != nil {
				return err
			}
		}
	default:
		return fmt.Errorf("portable values exclude floats, bytes and links")
	}
	return nil
}

func canonicalRaw(raw []byte) ([]byte, error) {
	n, err := jsonNode(raw)
	if err != nil {
		return nil, err
	}
	var out bytes.Buffer
	if err := dagcbor.Encode(n, &out); err != nil {
		return nil, err
	}
	return out.Bytes(), nil
}

// Canonical uses the official IPLD codec; this consumer defines no CBOR codec.
func Canonical(value any) ([]byte, error) {
	raw, err := strictjson.Marshal(value)
	if err != nil {
		return nil, err
	}
	return canonicalRaw(raw)
}
func Identity(value any) (string, error) {
	encoded, err := Canonical(value)
	if err != nil {
		return "", err
	}
	id, err := (cid.Prefix{Version: 1, Codec: cid.DagCBOR, MhType: mh.BLAKE3, MhLength: 32}).Sum(encoded)
	if err != nil {
		return "", err
	}
	return id.String(), nil
}
func equal(a, b any) bool {
	aa, e := Canonical(a)
	if e != nil {
		return false
	}
	bb, e := Canonical(b)
	return e == nil && bytes.Equal(aa, bb)
}
func rawPortable(raw json.RawMessage) error {
	n, e := jsonNode(raw)
	if e != nil {
		return e
	}
	return portable(n, 0, 32)
}

// ReadArtifact checks typed shape, canonical identity, and evidence consistency.
func ReadArtifact(raw []byte) (*Artifact, error) {
	if len(raw) > artifactLimit {
		return nil, fmt.Errorf("artifact exceeds 8 MiB")
	}
	original, err := canonicalRaw(raw)
	if err != nil {
		return nil, err
	}
	var a Artifact
	d := json.NewDecoder(bytes.NewReader(raw))
	d.DisallowUnknownFields()
	if err := d.Decode(&a); err != nil {
		return nil, err
	}
	encoded, err := Canonical(a)
	if err != nil {
		return nil, err
	}
	// Re-encoding detects missing required fields and null/type normalization.
	if !bytes.Equal(original, encoded) {
		return nil, fmt.Errorf("typed JSON shape does not round-trip")
	}
	if err := validate(a.Corpus); err != nil {
		return nil, err
	}
	id, err := Identity(a.Corpus)
	if err != nil {
		return nil, err
	}
	if id != a.ID {
		return nil, fmt.Errorf("content identity mismatch")
	}
	return &a, nil
}
