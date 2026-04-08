package sample

import "fmt"

const MaxRetries = 3

var DefaultTimeout = 30

type Config struct {
	Name  string
	Debug bool
}

type Processor interface {
	Process(data string) (string, error)
}

func NewConfig(name string) *Config {
	return &Config{Name: name}
}

func (c *Config) String() string {
	return fmt.Sprintf("Config{name=%s}", c.Name)
}
