#ifndef U_COMPONENT_LIST_H
#define U_COMPONENT_LIST_H

struct inventory;

typedef struct component component_t;
typedef struct transform transform_t;

struct component
{
	int id;
	float amount;

	char is_item;
	char kept;
};

struct transform
{
	int n_req;
	component_t* req;

	int n_res;
	component_t* res;

	float rate;
};

void transform_init(transform_t* t);
void transform_exit(transform_t* t);
void transform_copy(transform_t* t, transform_t* from);

void transform_req(transform_t* t, int id, float a, char is_item);
void transform_res(transform_t* t, int id, float a, char is_item);

int   transform_check(transform_t* t, struct inventory* inv);
float transform_ratio(transform_t* t, struct inventory* inv, float max_ratio);
float transform_apply(transform_t* t, struct inventory* inv, float ratio);

#endif
