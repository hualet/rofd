use serde::Deserialize;

/*
<?xml version="1.0" encoding="UTF-8"?><ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area>
    <ofd:PhysicalBox>0 0 211.50 140</ofd:PhysicalBox>
    <ofd:ApplicationBox>0 0 211.50 140</ofd:ApplicationBox>
  </ofd:Area>
  <ofd:Content>
    <ofd:Layer ID="2" Type="Body">
   <ofd:PathObject Boundary="10.30 30.30 0.30 22" ID="11" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 22</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="10 30 95.30 0.30" ID="12" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 95.30 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="105 30.30 0.30 22" ID="13" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 22</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="105 30 6.30 0.30" ID="14" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 6.30 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="111 30.30 0.30 22" ID="15" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 22</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="111 30 95.30 0.30" ID="16" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 95.30 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="206 30.30 0.30 22" ID="17" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 22</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="4 52 202.30 0.30" ID="18" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 202.30 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
            <ofd:PathObject Boundary="4 30 6.60 0.30" ID="9" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 6.60 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="4 30.30 0.30 22" ID="10" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 22</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
   </ofd:PathObject>
      <ofd:PathObject Boundary="4 52.30 0.30 45" ID="19" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 45</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="206 52.30 0.30 45" ID="20" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 45</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="4 97 52.60 0.30" ID="21" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 52.60 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="4 97.30 0.30 8" ID="22" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 8</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="56.30 97.30 0.30 8" ID="23" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 8</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="56 97 150.30 0.30" ID="24" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 150.30 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="206 97.30 0.30 8" ID="25" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 8</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="4 105 6.60 0.30" ID="26" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 6.60 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="4 105.30 0.30 20" ID="27" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 20</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="10.30 105.30 0.30 20" ID="28" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 20</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="10 105 196.30 0.30" ID="29" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 196.30 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="10 125.30 196.30 0.30" ID="30" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 196.30 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="206 105.30 0.30 20" ID="31" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0.15 0 L 0.15 20</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PathObject Boundary="4 125.30 6 0.30" ID="32" LineWidth="0.3">
        <ofd:AbbreviatedData>M 0 0.15 L 6 0.15</ofd:AbbreviatedData>
        <ofd:StrokeColor Value="128 0 0"/>
      </ofd:PathObject>
      <ofd:PageBlock ID="35">
        <ofd:ImageObject Boundary="7 6 20 20" CTM="20.50 0 0 20.50 0 0" ID="37" ResourceID="36"/>
      </ofd:PageBlock>
      <ofd:PageBlock ID="38">
        <ofd:TextObject Alpha="127" Boundary="207.50 30 20 20" CTM="0 1 -1 0 0.50 -2" Font="5" ID="39" Size="3.175">
          <ofd:TextCode DeltaX="3.18 3.18 3.18 3.18 3.18" X="1.50" Y="0">下载次数：1</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:TextObject Boundary="69 8 70 9" Font="5" ID="40" Size="7.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="7 7 7 7 7 7 7 7 7" X="0" Y="7">电子发票（普通发票）</ofd:TextCode>
      </ofd:TextObject>
      <ofd:PageBlock ID="41">
        <ofd:PathObject Boundary="0 0 211.50 140" ID="42" LineWidth="0.3" Stroke="true">
          <ofd:StrokeColor Value="128 0 0"/>
          <ofd:AbbreviatedData>M 66 19 L 139 19</ofd:AbbreviatedData>
        </ofd:PathObject>
        <ofd:PathObject Boundary="0 0 211.50 140" ID="43" LineWidth="0.3" Stroke="true">
          <ofd:StrokeColor Value="128 0 0"/>
          <ofd:AbbreviatedData>M 66 20 L 139 20</ofd:AbbreviatedData>
        </ofd:PathObject>
      </ofd:PageBlock>
      <ofd:TextObject Boundary="155 11.50 15 5" Font="5" ID="44" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 3 3 3" X="0" Y="3">发票号码：</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="170.5 11.5 27 5" Font="3" ID="45" Size="3.0">
        <ofd:TextCode DeltaX="1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35 1.35" X="0" Y="3"><![CDATA[25422000000003570205]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="155 17.50 15 5" Font="5" ID="46" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 3 3 3" X="0" Y="3">开票日期：</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="170.5 17.5 19.8 5" Font="3" ID="47" Size="3.0">
        <ofd:TextCode DeltaX="1.35 1.35 1.35 1.35 3.0 1.35 1.35 3.0 1.35 1.35" X="0" Y="3"><![CDATA[2025年01月06日]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:PageBlock ID="50">
        <ofd:TextObject Boundary="5.50 32.50 3.90 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="51" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="" X="0.53" Y="3.02">购</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:PageBlock ID="52">
        <ofd:TextObject Boundary="5.50 36 3.90 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="53" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="" X="0.53" Y="3.02">买</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:PageBlock ID="54">
        <ofd:TextObject Boundary="5.50 39.50 3.90 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="55" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="" X="0.53" Y="3.02">方</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:PageBlock ID="56">
        <ofd:TextObject Boundary="5.50 43 3.90 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="57" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="" X="0.53" Y="3.02">信</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:PageBlock ID="58">
        <ofd:TextObject Boundary="5.50 46.50 3.90 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="59" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="" X="0.53" Y="3.02">息</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:TextObject Boundary="11.50 33.50 7.50 5" Font="5" ID="60" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 3" X="0" Y="3">名称:</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="19.09 33.4 31.72 5.17" Font="3" ID="61" Size="3.1718">
        <ofd:TextCode DeltaX="3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718" X="0" Y="3.17"><![CDATA[统信软件技术有限公司]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:PageBlock ID="64">
        <ofd:TextObject Boundary="11 44 67.50 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="65" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="3.18 3.18 3.18 3.18 3.18 3.18 3.18 3.18 1.59 3.18 3.18 3.18 3.18 3.18 3.18" X="0.53" Y="3.02">统一社会信用代码/纳税人识别号：</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:TextObject Boundary="53.62 43.3 76.2 6.23" Font="7" ID="66" Size="4.2333">
        <ofd:TextCode DeltaX="g 18 2.54" X="0" Y="4.23"><![CDATA[91110302MA01NP925M]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:PageBlock ID="67">
        <ofd:TextObject Boundary="106 32.50 3.90 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="68" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="" X="0.53" Y="3.02">销</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:PageBlock ID="69">
        <ofd:TextObject Boundary="106 36 3.90 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="70" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="" X="0.53" Y="3.02">售</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:PageBlock ID="71">
        <ofd:TextObject Boundary="106 39.50 3.90 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="72" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="" X="0.53" Y="3.02">方</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:PageBlock ID="73">
        <ofd:TextObject Boundary="106 43 3.90 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="74" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="" X="0.53" Y="3.02">信</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:PageBlock ID="75">
        <ofd:TextObject Boundary="106 46.50 3.90 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="76" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="" X="0.53" Y="3.02">息</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:TextObject Boundary="111.50 33.50 7.94 5.18" Font="5" ID="77" Size="3.175">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3.18 3.18" X="0" Y="3.18">名称:</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="119.09 33.4 53.92 5.17" Font="3" ID="78" Size="3.1718">
        <ofd:TextCode DeltaX="3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718 3.1718" X="0" Y="3.17"><![CDATA[武汉东湖新技术开发区柴房老味餐饮店]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:PageBlock ID="81">
        <ofd:TextObject Boundary="111.50 44 67.50 3.30" CTM="0.89 0 0 1 0 0" Font="5" ID="82" Size="3.175">
          <ofd:FillColor Value="128 0 0"/>
          <ofd:TextCode DeltaX="3.18 3.18 3.18 3.18 3.18 3.18 3.18 3.18 1.59 3.18 3.18 3.18 3.18 3.18 3.18" X="0.53" Y="3.02">统一社会信用代码/纳税人识别号：</ofd:TextCode>
        </ofd:TextObject>
      </ofd:PageBlock>
      <ofd:TextObject Boundary="154.12 43.3 76.2 6.23" Font="7" ID="83" Size="4.2333">
        <ofd:TextCode DeltaX="g 18 2.54" X="0" Y="4.23"><![CDATA[92420100MA4JMP6D4P]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="21 93 15 5" Font="5" ID="84" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 1.50 1.50 1.50 1.50 1.50 1.50" X="0" Y="3">合      计</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="145.75 93 11.25 5" Font="3" ID="85" Size="3.0">
        <ofd:TextCode DeltaX="3.0 1.35 1.35 1.35 1.5 1.35" X="0" Y="3"><![CDATA[¥261.39]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="196.45 93 8.55 5" Font="3" ID="86" Size="3.0">
        <ofd:TextCode DeltaX="3.0 1.35 1.5 1.35" X="0" Y="3"><![CDATA[¥2.61]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="17 99 25.60 5" Font="5" ID="87" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3.20 3.20 3.20 3.20 3.20 3.20 3.20" X="0" Y="3">价税合计（大写）</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="59.5 99 25.5 5" Font="3" ID="88" Size="3.0">
        <ofd:TextCode DeltaX="3.0 1.5 3.0 3.0 3.0 3.0 3.0 3.0" X="0" Y="3"><![CDATA[ⓧ 贰佰陆拾肆圆整]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="144 99 12 5" Font="5" ID="89" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 3 3" X="0" Y="3">（小写）</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="157.5 99 11.25 5" Font="3" ID="90" Size="3.0">
        <ofd:TextCode DeltaX="3.0 1.35 1.35 1.35 1.5 1.35" X="0" Y="3"><![CDATA[¥264.00]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="5.50 110 3 5" Font="5" ID="91" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode X="0" Y="3">备</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="5.50 115 3 5" Font="5" ID="92" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode X="0" Y="3">注</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="20 130 15 5" Font="5" ID="99" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 1.50 3 1.50 3" X="0" Y="3">开 票 人：</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="36.5 130 9 5" Font="3" ID="100" Size="3.0">
        <ofd:TextCode DeltaX="3.0 3.0" X="0" Y="3"><![CDATA[郭正旭]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="16 53.50 16 5" Font="5" ID="101" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="4 4 4" X="0" Y="3">项目名称</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="43 53.50 14 5" Font="5" ID="102" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3.50 3.50 3.50" X="0" Y="3">规格型号</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="67.50 53.50 9 5" Font="5" ID="103" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 1.50 1.50" X="0" Y="3">单  位</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="93 53.50 9 5" Font="5" ID="104" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 1.50 1.50" X="0" Y="3">数  量</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="117.50 53.50 9 5" Font="5" ID="105" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 1.50 1.50" X="0" Y="3">单  价</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="143.50 53.50 9 5" Font="5" ID="106" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 1.50 1.50" X="0" Y="3">金  额</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="157 53.50 17.70 5" Font="5" ID="107" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3.20 3.20 1.70 3.20 3.20" X="0" Y="3">税率/征收率</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="194.50 53.50 9 5" Font="5" ID="108" Size="3.0">
        <ofd:FillColor Value="128 0 0"/>
        <ofd:TextCode DeltaX="3 1.50 1.50" X="0" Y="3">税  额</ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="6.5 57.5 24 5" Font="3" ID="109" Size="3.0">
        <ofd:TextCode DeltaX="1.5 3.0 3.0 3.0 3.0 1.5 3.0 3.0" X="0" Y="3"><![CDATA[*餐饮服务*餐饮费]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="146.25 57.5 8.25 5" Font="3" ID="114" Size="3.0">
        <ofd:TextCode DeltaX="1.35 1.35 1.35 1.5 1.35" X="0" Y="3"><![CDATA[261.39]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="165.83 57.5 2.85 5" Font="3" ID="115" Size="3.0">
        <ofd:TextCode DeltaX="1.35" X="0" Y="3"><![CDATA[1%]]></ofd:TextCode>
      </ofd:TextObject>
      <ofd:TextObject Boundary="199.45 57.5 5.55 5" Font="3" ID="116" Size="3.0">
        <ofd:TextCode DeltaX="1.35 1.5 1.35" X="0" Y="3"><![CDATA[2.61]]></ofd:TextCode>
      </ofd:TextObject>
    </ofd:Layer>
  </ofd:Content>
</ofd:Page>
*/

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum Event {
    PathObject(PathObject),
    TextObject(TextObject),
    PageBlock(PageBlock),
    ImageObject(ImageObject),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PathObject {
    #[serde(rename = "ID")]
    pub id: u32,
    pub boundary: String,
    pub line_width: f64,
    pub stroke: Option<bool>,
    pub stroke_color: Option<Color>,
    #[serde(rename = "CTM")]
    pub ctm: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Color {
    pub value: String,
    pub alpha: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ImageObject {
    #[serde(rename = "ID")]
    pub id: u32,
    pub boundary: String,
    #[serde(rename = "CTM")]
    pub ctm: String,
    #[serde(rename = "ResourceID")]
    pub resource_id: u32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PageBlock {
    #[serde(rename = "ID")]
    pub id: u32,
    #[serde(rename = "$value")]
    pub(crate) events: Vec<Event>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct TextObject {
    #[serde(rename = "ID")]
    pub id: u32,
    pub boundary: String,
    pub font: u32,
    pub size: f64,
    pub fill_color: Option<Color>,
    pub text_code: TextCode,
    #[serde(rename = "CTM")]
    pub ctm: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct TextCode {
    pub x: f64,
    pub y: f64,
    pub delta_x: Option<String>,
    #[serde(rename = "$value")]
    pub value: String,
}

impl Default for Color {
    fn default() -> Self {
        Color {
            value: "0 0 0".to_string(),
            alpha: Some(255.0),
        }
    }
}

/* Annot_0.xml
<?xml version="1.0" encoding="UTF-8"?>
<ofd:PageAnnot xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Annot Type="Stamp" Creator="OFD R&amp;W" LastModDate="2024-10-22" ID="173">
    <ofd:Appearance Boundary="87.50 8.50 30 20">
      <ofd:ImageObject ID="175" ResourceID="174" Boundary="0 0 30 20" CTM="30 0 0 20 0 0"/>
    </ofd:Appearance>
  </ofd:Annot>
</ofd:PageAnnot>
*/

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Annot {
    #[serde(rename = "ID")]
    pub id: u32,
    pub r#type: String,
    pub creator: String,
    pub last_mod_date: String,
    pub appearance: Appearance,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Appearance {
    pub boundary: String,
    pub image_object: ImageObject,
}
