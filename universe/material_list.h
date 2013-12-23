#ifndef U_MATERIAL_LIST_H
#define U_MATERIAL_LIST_H

typedef struct material_list material_list_t;

struct material_list
{
	int    length;
	int*   mat_ids;
	float* amounts;
};

void material_list_init(material_list_t* l);
void material_list_exit(material_list_t* l);

void  material_list_push (material_list_t* l, int id, float a);

int   material_list_check(material_list_t* l, const float* amounts);
float material_list_ratio(material_list_t* l, const float* amounts, float max_ratio);
void  material_list_apply(material_list_t* l,       float* amounts, float ratio);

#endif
