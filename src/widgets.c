/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#include "widgets.h"

#include <string.h>
#include <stdio.h>
#include <math.h>

void draw_cursor(graphics_t* gr, assets_t* a, int t)
{
	sfVector2i imouse = sfMouse_getPositionRenderWindow(gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
		sprite = assets_sprite(a, "data/cursors.png");
	sfIntRect rect = {24*t, 0, 24, 24};
	sfSprite_setTextureRect(sprite, rect);
	sfSprite_setPosition(sprite, mouse);
	sfRenderWindow_drawSprite(gr->render, sprite, NULL);
}

// inspired from https://github.com/LaurentGomila/SFML/wiki/Source%3A-Draw-Rounded-Rectangle
void draw_roundrect(graphics_t* gr, float x, float y, float w, float h)
{
	static sfVertexArray* array = NULL;
	static const int count = 8;
	static const float radius = 5;
#define M_PI 3.14159265358979323846
	if (array == NULL)
	{
		array = sfVertexArray_create();
		sfVertexArray_setPrimitiveType(array, sfTrianglesFan);
		sfVertexArray_resize(array, 4*count);
	}
	sfColor corners[4] =
	{
		{191, 191, 191, 191},
		{223, 223, 223, 191},
		{255, 255, 255, 191},
		{223, 223, 223, 191},
	};
	for (int i = 0; i < 4*count; i++)
	{
		sfVertex* v = sfVertexArray_getVertex(array, i);

		float deltaAngle = (M_PI/2.)/(count-1);
		int centerIndex = i / count;
		sfVector2f center =
		centerIndex == 0 ? (sfVector2f){w - radius,     radius} :
		centerIndex == 1 ? (sfVector2f){    radius,     radius} :
		centerIndex == 2 ? (sfVector2f){    radius, h - radius} :
		centerIndex == 3 ? (sfVector2f){w - radius, h - radius} :
		(sfVector2f){0,0};

		v->position.x = x+radius*cos(deltaAngle*(i-centerIndex)) + center.x;
		v->position.y = y+radius*sin(deltaAngle*(i-centerIndex)) - center.y + h;

		v->color = corners[centerIndex];
	}
	sfRenderWindow_drawVertexArray(gr->render, array, NULL);
}

void draw_tooltip(graphics_t* gr, assets_t* a, const char* txt)
{
	sfVector2i imouse = sfMouse_getPositionRenderWindow(gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont         (text, a->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, sfBlack);
	}

	sfVector2f pos = {mouse.x + 24, mouse.y + 24};
	sfText_setPosition(text, pos);
	sfText_setUTF8(text, txt);

	sfFloatRect rect = sfText_getGlobalBounds(text);
	rect.left   -= 3;
	rect.top    -= 3;
	rect.width  += 8;
	rect.height += 8;

	sfVector2u size = sfRenderWindow_getSize(gr->render);
	float dx = fmin(0, size.x - (rect.left+rect.width ));
	float dy = fmin(0, size.y - (rect.top +rect.height));
	pos.x += dx;
	pos.y += dy;
	rect.left += dx;
	rect.top  += dy;
	sfText_setPosition(text, pos);

	draw_roundrect(gr, rect.left, rect.top, rect.width, rect.height);

	sfRenderWindow_drawText(gr->render, text, NULL);
}

void draw_progressbar(graphics_t* gr, float x, float y, float w, float h, float p, char c)
{
	static sfRectangleShape* frame    = NULL;
	static sfRectangleShape* progress = NULL;
	if (frame == NULL)
	{
		frame = sfRectangleShape_create();
		sfRectangleShape_setFillColor(frame, sfTransparent);

		progress = sfRectangleShape_create();
	}

	float border = 1 + floor(h/20);
	sfRectangleShape_setOutlineThickness(frame, border);
	sfColor orange = {255, 42, 42, 255};
	sfRectangleShape_setOutlineColor(frame, c == 1 ? orange : sfWhite);

	sfColor inner;
	     if (c == -1)   inner = (sfColor){255,255,255,191};
	else if (c == -2)   inner = (sfColor){255,255,255,127};
	else if (c == -3)   inner = (sfColor){255,  0,  0,191};
	else if (c == -4)   inner = (sfColor){  0,  0,255,191};
	else if (p <= 0.25) inner = (sfColor){255,  0,  0,191};
	else if (p <= 0.50) inner = (sfColor){247,173,  0,191};
	else if (p <= 0.75) inner = (sfColor){170,170, 68,191};
	else if (p <= 1.00) inner = (sfColor){ 68,255, 68,191};
	else                inner = (sfColor){  0,  0,255,191};

	p = fmax(fmin(p, 1), 0);

	sfRectangleShape_setFillColor(progress, inner);

	sfVector2f pos = {x+border,y+border};
	sfRectangleShape_setPosition(frame, pos);
	sfRectangleShape_setPosition(progress, pos);

	sfVector2f size = {w-2*border, h-2*border};
	sfRectangleShape_setSize(frame, size);

	size.x *= p;
	sfRectangleShape_setSize(progress, size);
	sfRenderWindow_drawRectangleShape(gr->render, progress, NULL);
	sfRenderWindow_drawRectangleShape(gr->render, frame, NULL);
}

void draw_scrollbar(graphics_t* gr, float x, float y, float w, float h, float r, float p)
{
	static sfRectangleShape* cursor = NULL;
	if (cursor == NULL)
	{
		cursor = sfRectangleShape_create();
		sfRectangleShape_setFillColor(cursor, (sfColor){70,70,102,127});
	}

	r = fmin(r, 1);
	if (r != 1)
		y += p * h * (1-r);
	h *= r;
	sfRectangleShape_setSize(cursor, (sfVector2f){w,h});
	sfRectangleShape_setPosition(cursor, (sfVector2f){x,y});
	sfRenderWindow_drawRectangleShape(gr->render, cursor, NULL);
}

char draw_button(graphics_t* gr, assets_t* a, float x, float y, const char* name, char enabled, char do_draw)
{
	sfVector2i imouse = sfMouse_getPosition((sfWindow*) gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	static sfSprite* normal = NULL;
	static sfSprite* mini   = NULL;
	if (normal == NULL)
	{
		normal = assets_sprite(a, "data/menubutton.png");
		mini   = assets_sprite(a, "data/mini.png");
	}
	sfSprite* sprite = strlen(name) > 1 ? normal : mini;

	sfFloatRect rect = sfSprite_getGlobalBounds(sprite);
	sfVector2f pos = {x-rect.width/2, y-rect.height/2};
	sfSprite_setPosition(sprite, pos);

	char is_in = sfSprite_contains(sprite, mouse);
	sfColor color = enabled ? (is_in ? sfWhite : (sfColor){255,255,255,127}) : sfRed;
	sfSprite_setColor(sprite, color);

	if (do_draw)
		sfRenderWindow_drawSprite(gr->render, sprite, NULL);
	else
		return enabled && is_in;

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, a->font);
		sfText_setCharacterSize(text, 25);
	}

	sfText_setUTF8(text, name);
	rect = sfText_getLocalBounds(text);
	pos = (sfVector2f){floor(x-rect.width/2-rect.left), floor(y-rect.height/2-rect.top)};
	sfText_setPosition(text, pos);
	sfRenderWindow_drawText(gr->render, text, NULL);

	return 0;
}

char draw_toggle(graphics_t* gr, assets_t* a, float x, float y, const char* name, char* v, char do_draw)
{
	sfVector2i imouse = sfMouse_getPosition((sfWindow*) gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, a->font);
		sfText_setCharacterSize(text, 20);
	}

	{
		char buffer[1024];
		snprintf(buffer, 1024, "%s: %s", name, *v ? "oui" : "non");
		sfText_setUTF8(text, buffer);
		sfFloatRect rect = sfText_getLocalBounds(text);
		sfVector2f pos = {floor(x-rect.width/2-rect.left), floor(y-rect.height/2-rect.top)};
		sfText_setPosition(text, pos);
		if (do_draw)
			sfRenderWindow_drawText(gr->render, text, NULL);
		else if (sfText_contains(text, mouse))
			*v ^= 1;
	}

	return 0;
}

void draw_slider(graphics_t* gr, assets_t* a, float x, float y, const char* name, int* v, int min, int max, char do_draw)
{
	sfVector2i imouse = sfMouse_getPosition((sfWindow*) gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	{
		sfFloatRect rect = {0,0, 200, 40};
		rect.left = x - rect.width / 2;
		rect.top  = y - rect.height / 2;
		if (sfFloatRect_contains(&rect, mouse.x, mouse.y) && sfMouse_isButtonPressed(sfMouseLeft))
		{
			float r = (mouse.x - rect.left) / rect.width;
			*v = min + r * (max-min);
		}
		if (do_draw)
		{
			float r = (float)(*v - min) / (max-min);
			draw_progressbar(gr, rect.left, rect.top, rect.width, rect.height, r, -2);
		}
	}

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, a->font);
		sfText_setCharacterSize(text, 20);
	}

	char buffer[1024];
	snprintf(buffer, 1024, "%s: %i", name, *v);
	sfText_setUTF8(text, buffer);
	sfFloatRect rect = sfText_getLocalBounds(text);
	sfVector2f pos = {floor(x-rect.width/2-rect.left), floor(y-rect.height/2-rect.top)};
	sfText_setPosition(text, pos);
	if (do_draw)
		sfRenderWindow_drawText(gr->render, text, NULL);
}
