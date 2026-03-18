package entities;

import java.util.Objects;

public class Order {

    private String id;
    private String title;
    private BigDecimal price;
    private Boolean active;

    public Order() {}

    public String getId() { return id; }
    public void setId(String id) { this.id = id; }

    public String getTitle() { return title; }
    public void setTitle(String title) { this.title = title; }

    public BigDecimal getPrice() { return price; }
    public void setPrice(BigDecimal price) { this.price = price; }

    public Boolean getActive() { return active; }
    public void setActive(Boolean active) { this.active = active; }

    // <ai:customize hint="Add equals, hashCode, and business methods for Order">
    @Override
    public String toString() {
        return "Order{id=" + id + "}";
    }
    // </ai:customize>
}
