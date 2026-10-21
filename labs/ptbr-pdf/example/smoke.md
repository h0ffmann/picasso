# Reinício do serviço

Se o tempo de resposta passar de 500 ms por 5 min, o operador reinicia o serviço ⚠.

1. O operador lê os logs.
2. O operador reinicia o serviço com `systemctl restart app`.

| Passo | Quem | Prazo |
| :---- | :--- | :---- |
| ler os logs | operador | 2 min |
| reiniciar | operador | 1 min |

> Nota: ação, coração, pão, à, é, ç, ü — acentos, travessão e “aspas”.

```sh
systemctl status app
```

Mais em [picasso](https://github.com/h0ffmann/picasso).

```json
{"serviço": "app", "ativo": true, "tentativas": 3, "nota": null}
```

```python
def reiniciar(servico: str) -> bool:
    return servico == "app"  # ação
```

```sql
SELECT servico, COUNT(*) FROM logs WHERE nivel = 'erro' GROUP BY servico;
```

```rust
fn main() {
    let tempo: u32 = 500;
    println!("limite: {tempo} ms");
}
```

```scala
object Reinicio {
  def limite(ms: Int): Boolean = ms > 500
}
```
