#ifndef U_COMPONENT_LIST_H
#define U_COMPONENT_LIST_H

struct inventory;

typedef struct component  component_t;
typedef struct components components_t;

struct component
{
	int id;
	float amount;

	char is_item;
	char kept;
};

struct components
{
	int n;
	component_t* c;
};

void components_init(components_t* l);
void components_exit(components_t* l);
void components_copy(components_t* l, components_t* from);

void components_material(components_t* l, int id, float a);
void components_item    (components_t* l, int id, int   a);

int   components_check(components_t* l, struct inventory* inv);
float components_ratio(components_t* l, struct inventory* inv, float max_ratio);
void  components_apply(components_t* l, struct inventory* inv, float ratio);

#endif
