#ifndef U_COMPONENT_LIST_H
#define U_COMPONENT_LIST_H

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

void  components_push (components_t* l, int id, float a);

int   components_check(components_t* l, const float* amounts);
float components_ratio(components_t* l, const float* amounts, float max_ratio);
void  components_apply(components_t* l,       float* amounts, float ratio);

#endif
